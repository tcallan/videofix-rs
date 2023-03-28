use anyhow::anyhow;
use ffprobe::{FfProbe, Stream};
use itertools::Itertools;
use log::debug;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct FileMetadata {
    pub(crate) container: String,
    #[allow(unused)] // TODO: change to expect when available; for future functionality
    pub(crate) duration: Option<f64>,
    pub(crate) video: VideoMetadata,
    pub(crate) audio: AudioMetadata,
}

#[derive(Debug)]
pub(crate) struct VideoMetadata {
    #[allow(unused)] // TODO: change to expect when available; for future functionality
    pub(crate) index: i64,
    pub(crate) codec: String,
    pub(crate) pix_fmt: String,
}

#[derive(Debug)]
pub(crate) struct AudioMetadata {
    #[allow(unused)] // TODO: change to expect when available; for future functionality
    pub(crate) index: i64,
    pub(crate) codec: String,
    #[allow(unused)] // TODO: change to expect when available; for future functionality
    pub(crate) channels: i64,
}

pub(crate) fn get_metadata(path: impl AsRef<Path>) -> anyhow::Result<FileMetadata> {
    debug!("calling ffprobe");
    let details = ffprobe::ffprobe(&path)
        .map_err(|err| anyhow!("ffprobe error in {}: {}", path.as_ref().display(), err))?;
    debug!("ffprobe {:#?}", &details);
    let duration = details
        .format
        .duration
        .as_ref()
        .and_then(|d| d.parse::<f64>().ok())
        .map(|d| d / 60.0);

    Ok(FileMetadata {
        container: get_container(&details),
        duration,
        audio: get_audio_metadata(&details)?,
        video: get_video_metadata(&details)?,
    })
}

fn get_container(details: &FfProbe) -> String {
    details
        .format
        .format_name
        .chars()
        .take_while(|&c| c != ',')
        .collect()
}

fn get_video_metadata(details: &FfProbe) -> anyhow::Result<VideoMetadata> {
    let video_stream = find_stream_by_type(details, "video")?;

    debug!("video {:#?}", video_stream);

    Ok(VideoMetadata {
        index: video_stream.index,
        codec: get_codec(video_stream)?,
        pix_fmt: get_pix_fmt(video_stream)?,
    })
}

fn get_audio_metadata(details: &FfProbe) -> anyhow::Result<AudioMetadata> {
    let audio_stream = find_stream_by_type(details, "audio")?;

    debug!("audio {:#?}", audio_stream);

    Ok(AudioMetadata {
        index: audio_stream.index,
        codec: get_codec(audio_stream)?,
        channels: audio_stream.channels.unwrap_or(0),
    })
}

fn find_stream_by_type<'a>(details: &'a FfProbe, stream_type: &str) -> anyhow::Result<&'a Stream> {
    details
        .streams
        .iter()
        .filter(|&s| {
            s.codec_type
                .as_ref()
                .map(|s| s == stream_type)
                .unwrap_or_else(|| false)
        })
        .at_most_one()
        .map_err(|_| {
            anyhow!(
                "more than one matching {} stream in {}",
                stream_type,
                details.format.filename
            )
        })
        .and_then(|maybe_stream| {
            maybe_stream.ok_or_else(|| {
                anyhow!(
                    "no {} stream found in {}",
                    stream_type,
                    details.format.filename
                )
            })
        })
}

fn get_codec(stream: &Stream) -> anyhow::Result<String> {
    stream
        .codec_name
        .as_ref()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("no codec found for stream {}", stream.index))
}

fn get_pix_fmt(stream: &Stream) -> anyhow::Result<String> {
    stream
        .pix_fmt
        .as_ref()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("no pix_fmt found for stream {}", stream.index))
}
