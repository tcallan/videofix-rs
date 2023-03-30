use crate::metadata;

use super::FormatSpec;
use super::Formats;

#[derive(Debug)]
pub(crate) struct FormatValidation {
    pub(crate) audio_okay: bool,
    pub(crate) video_okay: bool,
    pub(crate) container_okay: bool,
    pub(crate) pix_fmt_okay: bool,
}

impl FormatValidation {
    pub(crate) fn is_valid(&self) -> bool {
        self.audio_okay && self.video_okay && self.container_okay && self.pix_fmt_okay
    }
}

pub(crate) fn validate_format(
    file: &metadata::FileMetadata,
    format: &FormatSpec,
) -> FormatValidation {
    let audio_okay = validate_format_component(&format.audio, &file.audio.codec);
    let video_okay = validate_format_component(&format.video, &file.video.codec);
    let container_okay = validate_format_component(&format.container, &file.container);
    let pix_fmt_okay = validate_format_component(&format.pix_fmt, &file.video.pix_fmt);

    FormatValidation {
        audio_okay,
        video_okay,
        container_okay,
        pix_fmt_okay,
    }
}

fn validate_format_component(format: &Formats, value: &String) -> bool {
    match format {
        Formats::Allow(items) => allow(items, value),
        Formats::Reject(items) => reject(items, value),
    }
}

fn allow(format: &[String], value: &String) -> bool {
    format.contains(value)
}

fn reject(format: &[String], value: &String) -> bool {
    !allow(format, value)
}

#[cfg(test)]
mod test {
    use itertools::Itertools;

    use super::*;
    use crate::metadata::{AudioMetadata, FileMetadata, VideoMetadata};

    fn str_vec(v: Vec<&str>) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect_vec()
    }

    fn mk_metadata(container: &str, vcodec: &str, acodec: &str) -> FileMetadata {
        FileMetadata {
            container: container.to_string(),
            duration: None,
            video: VideoMetadata {
                index: 0,
                codec: vcodec.to_string(),
                pix_fmt: "".to_string(),
            },
            audio: AudioMetadata {
                index: 1,
                codec: acodec.to_string(),
                channels: 2,
            },
        }
    }

    fn mk_spec_allow(audio: Vec<&str>, video: Vec<&str>, container: Vec<&str>) -> FormatSpec {
        FormatSpec {
            audio: Formats::Allow(str_vec(audio)),
            video: Formats::Allow(str_vec(video)),
            container: Formats::Allow(str_vec(container)),
            pix_fmt: Formats::Reject(vec![]),
        }
    }

    fn mk_spec_reject(audio: Vec<&str>, video: Vec<&str>, container: Vec<&str>) -> FormatSpec {
        FormatSpec {
            audio: Formats::Reject(str_vec(audio)),
            video: Formats::Reject(str_vec(video)),
            container: Formats::Reject(str_vec(container)),
            pix_fmt: Formats::Reject(vec![]),
        }
    }

    #[test]
    fn format_validation_allow_valid() {
        let format = mk_spec_allow(vec!["mp3", "wav"], vec!["h264", "h265"], vec!["avi", "mp4"]);
        let metadata = mk_metadata("mp4", "h265", "mp3");

        let validation = validate_format(&metadata, &format);
        assert!(validation.is_valid());
    }

    #[test]
    fn format_validation_allow_invalid_container() {
        let format = mk_spec_allow(vec!["mp3", "wav"], vec!["h264", "h265"], vec!["avi", "mp4"]);
        let metadata = mk_metadata("mkv", "h265", "mp3");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_allow_invalid_video() {
        let format = mk_spec_allow(vec!["mp3", "wav"], vec!["h264", "h265"], vec!["avi", "mp4"]);
        let metadata = mk_metadata("mp4", "avi", "mp3");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_allow_invalid_audio() {
        let format = mk_spec_allow(vec!["mp3", "wav"], vec!["h264", "h265"], vec!["avi", "mp4"]);
        let metadata = mk_metadata("mp4", "h265", "flac");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_reject_valid() {
        let format = mk_spec_reject(vec!["mp3", "wav"], vec!["h264", "h265"], vec!["avi", "mp4"]);
        let metadata = mk_metadata("mkv", "mp4", "aac");

        let validation = validate_format(&metadata, &format);
        assert!(validation.is_valid());
    }

    #[test]
    fn format_validation_reject_invalid_container() {
        let format = mk_spec_reject(vec!["mp3", "wav"], vec!["h264", "h265"], vec!["avi", "mp4"]);
        let metadata = mk_metadata("avi", "mp4", "aac");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_reject_invalid_video() {
        let format = mk_spec_reject(vec!["mp3", "wav"], vec!["h264", "h265"], vec!["avi", "mp4"]);
        let metadata = mk_metadata("mkv", "h264", "aac");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_reject_invalid_audio() {
        let format = mk_spec_reject(vec!["mp3", "wav"], vec!["h264", "h265"], vec!["avi", "mp4"]);
        let metadata = mk_metadata("mkv", "mp4", "mp3");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }
}
