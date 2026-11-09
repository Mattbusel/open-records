//! open-records: pull US public records straight from the agencies into a local SQLite file.
//! No account, no API key, no broker in the middle.
//!
//!   open-records edgar <ticker-or-name-or-CIK>      recent SEC filings for a company
//!   open-records awards <recipient name>            federal contract awards (USAspending) by recipient
//!   open-records show [filings|awards]              print what is stored locally
//!
//! Data goes to ./open-records.sqlite (override with OPEN_RECORDS_DB).

use rusqlite::{params, Connection};
use serde_json::{json, Value};

const UA: &str = "open-records Matthew Busel mattbusel@tensorust.com";

fn db() -> rusqlite::Result<Connection> {
    let path = std::env::var("OPEN_RECORDS_DB").unwrap_or_else(|_| "open-records.sqlite".into());
    let c = Connection::open(path)?;
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS filings(cik TEXT, company TEXT, accession TEXT PRIMARY KEY, form TEXT, filed TEXT, doc TEXT, url TEXT);
         CREATE TABLE IF NOT EXISTS awards(award_id TEXT PRIMARY KEY, recipient TEXT, agency TEXT, amount REAL, start_date TEXT, end_date TEXT, description TEXT);",
    )?;
    Ok(c)
}

fn get(url: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(ureq::get(url).set("User-Agent", UA).call()?.into_json()?)
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn edgar(q: &str) -> Result<(), Box<dyn std::error::Error>> {
    // The SEC publishes a ticker/name/CIK index; resolve the query locally against it.
    let idx = get("https://www.sec.gov/files/company_tickers.json")?;
    let ql = q.to_lowercase();
    let hit = idx
        .as_object()
        .ok_or("unexpected index shape")?
        .values()
        .find(|e| {
            s(e, "ticker").to_lowercase() == ql
                || e.get("cik_str").map(|c| c.to_string() == q).unwrap_or(false)
                || s(e, "title").to_lowercase().contains(&ql)
        })
        .ok_or("no company matched; try a ticker like AAPL or a CIK number")?
        .clone();
    let cik = hit["cik_str"].as_u64().unwrap_or(0);
    let name = s(&hit, "title");
    let sub = get(&format!("https://data.sec.gov/submissions/CIK{:010}.json", cik))?;
    let r = &sub["filings"]["recent"];
    let n = r["accessionNumber"].as_array().map(|a| a.len()).unwrap_or(0);
    let c = db()?;
    let mut stored = 0;
    for i in 0..n {
        let acc = r["accessionNumber"][i].as_str().unwrap_or("");
        let doc = r["primaryDocument"][i].as_str().unwrap_or("");
        let url = format!(
            "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
            cik,
            acc.replace('-', ""),
            doc
        );
        c.execute(
            "INSERT OR REPLACE INTO filings VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![cik.to_string(), name, acc, r["form"][i].as_str().unwrap_or(""), r["filingDate"][i].as_str().unwrap_or(""), doc, url],
        )?;
        stored += 1;
    }
    println!("{} (CIK {}): stored {} recent filings in the local database", name, cik, stored);
    Ok(())
}

fn awards(q: &str) -> Result<(), Box<dyn std::error::Error>> {
    let body = json!({
        "filters": {"recipient_search_text": [q], "award_type_codes": ["A","B","C","D"]},
        "fields": ["Award ID","Recipient Name","Awarding Agency","Award Amount","Start Date","End Date","Description"],
        "sort": "Award Amount", "order": "desc", "limit": 50, "page": 1
    });
    let resp: Value = ureq::post("https://api.usaspending.gov/api/v2/search/spending_by_award/")
        .set("User-Agent", UA)
        .send_json(body)?
        .into_json()?;
    let c = db()?;
    let mut stored = 0;
    let mut total = 0.0;
    for a in resp["results"].as_array().unwrap_or(&vec![]) {
        let amt = a["Award Amount"].as_f64().unwrap_or(0.0);
        total += amt;
        c.execute(
            "INSERT OR REPLACE INTO awards VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![s(a, "Award ID"), s(a, "Recipient Name"), s(a, "Awarding Agency"), amt, s(a, "Start Date"), s(a, "End Date"), s(a, "Description")],
        )?;
        stored += 1;
    }
    println!("stored {} awards matching \"{}\" ({:.0} USD across them)", stored, q, total);
    Ok(())
}

fn show(what: &str) -> Result<(), Box<dyn std::error::Error>> {
    let c = db()?;
    if what == "awards" {
        let mut st = c.prepare("SELECT recipient, agency, amount, start_date FROM awards ORDER BY amount DESC LIMIT 20")?;
        for r in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?, r.get::<_, String>(3)?)))? {
            let (a, b, amt, d) = r?;
            println!("{:>14.0}  {}  {}  {}", amt, d, a, b);
        }
    } else {
        let mut st = c.prepare("SELECT company, form, filed, url FROM filings ORDER BY filed DESC LIMIT 20")?;
        for r in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)))? {
            let (co, f, d, u) = r?;
            println!("{}  {:<8} {}  {}", d, f, co, u);
        }
    }
    Ok(())
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let res = match (a.get(0).map(|x| x.as_str()), a.get(1)) {
        (Some("edgar"), Some(q)) => edgar(q),
        (Some("awards"), Some(_)) => awards(&a[1..].join(" ")),
        (Some("show"), w) => show(w.map(|x| x.as_str()).unwrap_or("filings")),
        _ => {
            eprintln!("usage:\n  open-records edgar <ticker|name|CIK>\n  open-records awards <recipient name>\n  open-records show [filings|awards]");
            std::process::exit(2);
        }
    };
    if let Err(e) = res {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
