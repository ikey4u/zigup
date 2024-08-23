use anyhow::Context;
use reqwest::blocking::Response;

use crate::Result;

pub fn request<S1: AsRef<str>, S2: AsRef<str>>(
    url: S1,
    proxy: Option<S2>,
) -> Result<Response> {
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(proxy) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy.as_ref())?);
    }
    let client = builder.build()?;
    let url = url.as_ref();
    let resp = client.get(url).send().context(format!("GET {url}"))?;
    Ok(resp)
}
