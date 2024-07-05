// see also: num-format

use reqwest::Url;

use crate::LASTFM_URL;

const MILLION: f64 = 1_000_000.0;

pub fn human_number(num: u32) -> String {
    match (num as f64).log10().floor() as usize {
        0..=2 => num.to_string(),
        3..=5 => format!("{} K", num / 1000),
        // 3 significant figures
        6..=8 => format!("{} M", {
            let num = ((num as f64) / MILLION).to_string();
            num[..num.len().min(4)].to_string().trim_end_matches('0')
        }),
        _ => todo!(),
    }
}

/// Because repeatedly using `format!` is annoying
///
/// Note: values in `params` must not be URL encoded!
pub fn build_lastfm_url(
    method: &str,
    key: &str,
    params: &[(&str, &str)],
) -> anyhow::Result<Url> {
    let mut all_params = vec![("method", method), ("api_key", key)];
    all_params.extend_from_slice(params);
    let url = Url::parse_with_params(&LASTFM_URL, all_params)?;
    Ok(url)
}

#[cfg(test)]
mod tests {
    use crate::utils::human_number;

    #[test]
    fn test_human_number() {
        for (num, str) in [
            (1, "1"),
            (2, "2"),
            (10, "10"),
            (100, "100"),
            (1000, "1 K"),
            (10000, "10 K"),
            (999999, "999 K"),
            (1_000_000, "1 M"),
            (1_100_000, "1.1 M"),
            (1_100_001, "1.1 M"),
        ] {
            assert_eq!(human_number(num), str)
        }
    }
}
