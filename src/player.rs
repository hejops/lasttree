use anyhow::Context;
use youtube_dl::SearchOptions;
use youtube_dl::YoutubeDl;

pub async fn search_youtube(query: &str) -> anyhow::Result<String> {
    let opts = SearchOptions::youtube(query);
    let pl = YoutubeDl::search_for(&opts)
        .run_async()
        .await
        .unwrap()
        .into_playlist()
        .unwrap()
        .entries
        .unwrap();
    // println!("{:#?}", pl);
    let first = pl
        .iter()
        .filter(|f| f.webpage_url.is_some())
        .map(|f| f.clone().webpage_url.unwrap())
        .next()
        .unwrap();
    // println!("{:?}", res);
    get_youtube_audio_link(&first).await
}

pub async fn get_youtube_audio_link(url: &str) -> anyhow::Result<String> {
    // yt[m] -> extract UC -> pass to /feeds/ -> extract link rel -> extract audio
    // url (requires decryption, which is not trivial!)
    // https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/youtube.py#L3854

    // r#"https://music.youtube.com/channel[^"?]+"#
    // "https://www.youtube.com/feeds/videos.xml?channel_id={id}"

    let link = YoutubeDl::new(url)
        .socket_timeout("15")
        .run_async()
        .await?
        .into_single_video()
        .context("into_single_video")?
        .formats
        .context("formats")?
        .into_iter()
        .filter_map(|f| f.url)
        .find(|s| s.contains("audio") && !s.contains("manifest"))
        .context("formats")?;
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
