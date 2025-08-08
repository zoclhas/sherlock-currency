use chrono::{NaiveDate, Utc};
use dirs::{self, cache_dir};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::{self, OpenOptions},
    path::Path,
};

#[derive(Debug, Deserialize, Serialize)]
struct JsonResponse {
    date: String,
    usd: HashMap<String, f64>,

    #[serde(default)]
    last_currency: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SherlockResponse {
    title: String,
    content: String,
    next_content: String,
}
impl SherlockResponse {
    fn show(&self) {
        let s = serde_json::to_string(self).unwrap();
        println!("{s}");
    }
}

async fn update_rates(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let exists = Path::new(&path).exists();
    let mut last_currency: String = "eur".into();

    if !exists {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .unwrap();
    } else {
        let cached_data: JsonResponse = serde_json::from_str(&fs::read_to_string(path).unwrap())?;
        last_currency = cached_data.last_currency;
    }

    let url =
        "https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/usd.json";
    let mut res: JsonResponse = surf::get(url).recv_json().await?;
    res.last_currency = last_currency;
    res.usd.insert("usd".into(), 1.00);

    let data = serde_json::to_string(&res)?;
    fs::write(path, data)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cache_path = cache_dir().unwrap();
    cache_path.push("sherlock-currency");
    if !Path::new(&cache_path).exists() {
        match fs::create_dir(&cache_path) {
            Ok(_) => (),
            Err(e) => return Err(format!("Unable to access cache. {e}").into()),
        };
    }
    cache_path.push("rates.json");

    if !Path::new(&cache_path).exists() {
        update_rates(&cache_path).await?;
    }

    let mut cached_data: JsonResponse =
        serde_json::from_str(&fs::read_to_string(&cache_path).unwrap())?;
    let last_currency = cached_data.last_currency;

    let last_date = NaiveDate::parse_from_str(&cached_data.date, "%Y-%m-%d").unwrap();
    let today = Utc::now().date_naive();
    if (today - last_date).num_days() > 7 {
        update_rates(&cache_path).await?;
    }

    let args: Vec<String> = env::args().skip(1).collect();
    let input = &args.join(" ");

    let re = Regex::new(
        r"(?x)
            ^\s*
            [\$]?
            (?P<amount>\d+(?:\.\d+)?)
            [\s\$]*
            (?P<from>[a-z]{3})?
            (?:\s+to\s+(?P<to>[a-z]{3}))?
            \s*$
            ",
    )
    .unwrap();

    if let Some(caps) = re.captures(input) {
        let amount: f64 = caps.name("amount").unwrap().as_str().parse().unwrap();
        let from_currency = caps.name("from").map(|m| m.as_str()).unwrap_or("usd");
        let to_currency = caps
            .name("to")
            .map(|m| m.as_str())
            .unwrap_or(&last_currency);

        let content = format!(
            "{} {:.2} to {}",
            from_currency.to_uppercase(),
            amount,
            to_currency.to_uppercase()
        );
        let title: String;

        if let Some(rate) = cached_data.usd.get(to_currency) {
            title = format!("{} {:.2}", to_currency.to_uppercase(), amount * rate);
            let r = SherlockResponse {
                title,
                content,
                next_content: String::new(),
            };
            r.show();

            let last_curr = String::from(to_currency);
            cached_data.last_currency = last_curr;
            let data = serde_json::to_string(&cached_data)?;
            fs::write(cache_path, data)?;
        } else {
            title = "Conversion rate not found.".into();
            let r = SherlockResponse {
                title,
                content,
                next_content: String::new(),
            };
            r.show();
        }
    } else {
        let r = SherlockResponse {
            title: input.into(),
            content: "Converting...".into(),
            next_content: String::new(),
        };
        r.show();
    }

    Ok(())
}
