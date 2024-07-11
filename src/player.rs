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
use youtube_dl::SingleVideo;
use youtube_dl::YoutubeDl;

#[derive(Debug)]
pub struct YoutubeAudio {
    /// Decrypted googlevideo link
    pub link: String,
    pub uploader: String,
    pub title: String,
}

pub async fn search_youtube(query: &str) -> anyhow::Result<YoutubeAudio> {
    let n = 5;
    let opts = SearchOptions::youtube(query).with_count(n);
    let results = YoutubeDl::search_for(&opts)
        .run_async()
        .await?
        .into_playlist()
        .context(format!("No search results for {query}."))?
        .entries
        .context("'entries' field empty")?;

    let first = results
        .iter()
        .find(|r| {
            r.categories
                .as_ref()
                .unwrap()
                .contains(&Some("Music".to_owned()))
        })
        .context(format!("No audio in first {n} results for {query}."))?;

    get_youtube_audio_link(first.clone()).await
}

/// Transform `SingleVideo` into a simpler `YoutubeAudio` struct. No requests
/// are made.
async fn get_youtube_audio_link(vid: SingleVideo) -> anyhow::Result<YoutubeAudio> {
    // r#"https://music.youtube.com/channel[^"?]+"#
    // "https://www.youtube.com/feeds/videos.xml?channel_id={id}"

    // println!("{:#?}", vid);

    let link = vid
        .formats
        .context("'formats' field empty")?
        .into_iter()
        .filter_map(|f| f.url)
        .find(|s| s.contains("audio") && !s.contains("manifest"))
        .context("no audio formats")?;

    let uploader = vid.uploader.context("'uploader' field empty")?;
    let title = vid.title.context("'title' field empty")?;

    let audio = YoutubeAudio {
        link,
        uploader,
        title,
    };
    Ok(audio)
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn search_artist_with_title() {
        let audio = crate::player::search_youtube("death grips warping")
            .await
            .unwrap();
        // println!("{:?}", audio);
        assert_eq!(reqwest::get(&audio.link).await.unwrap().status(), 200);
        assert!(audio.link.contains("mime=audio"));

        // microsecond can and will vary!
        assert!(audio.link.contains("dur=174."), "{}", audio.link);
    }

    #[tokio::test]
    async fn search_artist_only() {
        // a rather poor test, often not deterministic
        let audio = crate::player::search_youtube("death grips").await.unwrap();
        assert_eq!(reqwest::get(&audio.link).await.unwrap().status(), 200);
        assert_eq!(audio.uploader, "Death Grips");
    }
}
