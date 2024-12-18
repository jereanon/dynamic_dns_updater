use digitalocean::DigitalOcean;
use digitalocean::api::{Domain, DomainRecord};
use std::env;
use std::error::Error;
use digitalocean::prelude::Executable;
use tokio::time::{sleep, Duration};
use reqwest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_token = env::var("DO_TOKEN").expect("DO_TOKEN environment variable not set");
    let domain_name = env::var("DOMAIN_NAME").expect("DOMAIN_NAME environment variable not set");
    let subdomain = env::var("SUBDOMAIN").expect("SUBDOMAIN environment variable not set");

    let check_interval = Duration::from_secs(300); // 5 minutes

    let client = DigitalOcean::new(api_token).unwrap();
    let mut last_ip = String::new();

    println!("starting dynamic dns server for: {}.{}", subdomain, domain_name);

    loop {
        match update_dns(&client, domain_name.as_str(), subdomain.as_str(), &mut last_ip).await {
            Ok(_) => println!("dynamic dns check complete"),
            Err(e) => eprintln!("error updating DNS: {}", e),
        }

        sleep(check_interval).await;
    }
}

async fn get_public_ip() -> Result<String, Box<dyn Error>> {
    let response = reqwest::get("https://api.ipify.org").await?;
    let ip = response.text().await?;
    Ok(ip)
}

async fn update_dns(
    client: &DigitalOcean,
    domain_name: &str,
    subdomain: &str,
    last_ip: &mut String,
) -> Result<(), Box<dyn Error>> {
    let current_ip = get_public_ip().await?;

    // only update if necessary
    if current_ip == *last_ip {
        println!("ip unchanged, no need to update: ({})", current_ip);
        return Ok(());
    }

    println!("new IP address detected {}", current_ip);

    // get domain records for domain and find our subdomain record
    let records = Domain::get(domain_name).records().execute(client).unwrap();
    let subdomain_record = records.iter().find(|r| {
        r.name() == subdomain && (r.kind() == "A" || r.kind() == "CNAME")
    });

    println!("subdomain record found: {:?}", subdomain_record);

    match subdomain_record {
        Some(record) => {
            Domain::get(domain_name).records().update(*subdomain_record.unwrap().id()).data(current_ip.clone()).execute(client).unwrap();
            println!("updated existing DNS record");
        },
        None => {
            Domain::get(domain_name).records().create("A", subdomain, current_ip.clone().as_str()).execute(client).unwrap();
            println!("created new DNS record");
        }
    }

    *last_ip = current_ip;
    Ok(())
}
