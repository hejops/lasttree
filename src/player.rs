//! Module for sending simple search queries to YouTube and retrieving
//! audio-only URLs (googlevideo). `yt-dlp` (or similar) is a required
//! dependency.

// The full sequence is as follows:
//
// 1. search YouTube with some query
// 2. extract first UC... channel_id (buried in a script, but doable with regex)
// 3. pass to channel_id to /feeds/
// 4. parse xml (relatively easy), extract first "link rel"
// 5. parse YouTube source for a googlevideo URL (again, somewhat doable with
//    regex)
// 6. decrypt the URL -- this is basically where i throw in the towel, as decryption is not at all
//    trivial: https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/youtube.py#L3854

use anyhow::Context;
use youtube_dl::SearchOptions;
use youtube_dl::YoutubeDl;

pub async fn search_youtube(query: &str) -> anyhow::Result<String> {
    let opts = SearchOptions::youtube(query);
    let pl = YoutubeDl::search_for(&opts)
        .run_async()
        .await?
        .into_playlist()
        .context("no search results")?
        .entries
        .context("'entries' field empty")?;
    // println!("{:#?}", pl);
    let first = pl
        .into_iter()
        .filter_map(|f| f.webpage_url)
        .next()
        .context("no search results")?;
    // println!("{:?}", res);
    get_youtube_audio_link(&first).await
}

async fn get_youtube_audio_link(url: &str) -> anyhow::Result<String> {
    // r#"https://music.youtube.com/channel[^"?]+"#
    // "https://www.youtube.com/feeds/videos.xml?channel_id={id}"

    let link = YoutubeDl::new(url)
        .socket_timeout("15")
        .run_async()
        .await?
        .into_single_video()
        .context("could not extract single video")?
        .formats
        .context("'formats' field empty")?
        .into_iter()
        .filter_map(|f| f.url)
        .find(|s| s.contains("audio") && !s.contains("manifest"))
        .context("no audio formats")?;
    Ok(link)
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn extract_googlevideo() {
        let url = "https://youtube.com/watch?v=nHYOoyksB_I";
        let audio = crate::player::get_youtube_audio_link(url).await.unwrap();
        // println!("{:?}", audio);
        assert_eq!(reqwest::get(&audio).await.unwrap().status(), 200);
        assert!(audio.contains("mime=audio"));
        assert!(audio.contains("dur=174.278")); // microsecond can vary?
    }

    #[tokio::test]
    async fn search() {
        let audio = crate::player::search_youtube("death grips warping")
            .await
            .unwrap();
        // println!("{:?}", audio);
        assert_eq!(reqwest::get(&audio).await.unwrap().status(), 200);
    }
}
