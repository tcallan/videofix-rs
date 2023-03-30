use std::{
    env,
    ffi::OsStr,
    fs,
    io::stdin,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, bail, Context};
use clap::Parser;
use directories::ProjectDirs;
use env_logger::Builder;
use log::{debug, LevelFilter};
use metadata::FileMetadata;
use serde::{Deserialize, Serialize};
use terminal_size::{terminal_size, Width};
use validation::FormatValidation;

mod metadata;
mod validation;

const VALID_EXTENSIONS: [&str; 6] = ["mkv", "mp4", "avi", "webm", "mov", "wmv"];

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    #[arg(long)]
    fix: bool,
    #[arg(long)]
    target: Option<String>,
    path: Option<PathBuf>,
    #[arg(long)]
    debug: bool,
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    Builder::new()
        .filter_level(if args.debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Warn
        })
        .init();

    let config = load_config(args.config)?;

    let check_path = args
        .path
        .ok_or_else(|| anyhow!("no path"))
        .or_else(|_| env::current_dir())?;

    let should_fix = args.fix;

    let requested_target = args.target.as_ref().unwrap_or(&config.default_target);
    let target = config.find_target(requested_target)?;

    let mut check_paths: Vec<PathBuf> = Vec::new();

    if check_path.is_file() {
        check_paths.push(check_path);
    } else {
        get_paths(&check_path, &mut check_paths)?;
    }

    println!(
        "Checking {} against target \"{}\"",
        check_paths.len(),
        requested_target
    );
    for path in check_paths {
        // TODO: prompt before reencoding?
        handle_file(path, target, should_fix)?;
    }

    Ok(())
}

fn get_paths(check_path: &Path, check_paths: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let paths = fs::read_dir(check_path)?;
    let extensions = VALID_EXTENSIONS.map(OsStr::new);
    for entry in paths.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extensions.contains(&extension) {
                    check_paths.push(path);
                }
            }
        }
    }
    Ok(())
}

fn load_config(config_override: Option<PathBuf>) -> anyhow::Result<Config> {
    // TODO: could create a default placeholder config if one doesn't exist and prompt to edit
    let paths = ProjectDirs::from("", "", "videofix")
        .ok_or_else(|| anyhow!("could not determine program config directory"))?;

    let config_file = config_override.unwrap_or_else(|| paths.config_dir().join("config.gura"));

    let gura = fs::read_to_string(&config_file)
        .with_context(|| format!("could not load {}", config_file.display()))?;

    let config: Config =
        serde_gura::from_str(&gura).with_context(|| "could not deserialize config")?;
    Ok(config)
}

fn handle_file(path: PathBuf, target: &Target, should_fix: bool) -> anyhow::Result<()> {
    let metadata = metadata::get_metadata(&path)?;
    let validation = validation::validate_format(&metadata, &target.format_spec);

    report(&path, &metadata, &validation);

    if !validation.is_valid() && should_fix {
        reencode(&path, &validation, &target.default)?;
    };
    Ok(())
}

fn report(path: &Path, metadata: &FileMetadata, validation: &FormatValidation) {
    println!();
    println!(
        "{}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("..")
    );
    println!(
        " - {} {}; {} {}; {} {}; {} {}",
        metadata.audio.codec,
        report_status(validation.audio_okay),
        metadata.video.codec,
        report_status(validation.video_okay),
        metadata.container,
        report_status(validation.container_okay),
        metadata.video.pix_fmt,
        report_status(validation.pix_fmt_okay),
    );
}

fn report_status(is_okay: bool) -> &'static str {
    if is_okay {
        "✅"
    } else {
        "❌"
    }
}

fn reencode(
    in_path: impl AsRef<Path>,
    val: &FormatValidation,
    default: &DefaultFormat,
) -> anyhow::Result<()> {
    let vcodec = if val.video_okay {
        "copy"
    } else {
        &default.video
    };
    let acodec = if val.audio_okay {
        "copy"
    } else {
        &default.audio
    };

    let out_path = in_path.as_ref().with_extension("fixed.mkv");

    // TODO: could let ffmepg prompt for this instead
    if out_path.exists() {
        bail!("fix target {} already exists", out_path.display());
    }

    guard_terminal_size(100);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-loglevel")
        .arg("warning")
        .arg("-stats")
        .arg("-i")
        .arg(in_path.as_ref())
        .arg("-c:v")
        .arg(vcodec);

    if !val.pix_fmt_okay {
        cmd.arg("-pix_fmt").arg(&default.pix_fmt);
    }

    cmd.arg("-c:a").arg(acodec).arg(out_path);

    debug!("{:?}", cmd);

    let mut ffmpeg = cmd.spawn()?;

    ffmpeg.wait()?;

    Ok(())
}

fn guard_terminal_size(min_width: u16) {
    if let Some((Width(w), _)) = terminal_size() {
        if w < min_width {
            println!("Terminal width is below minimum size for nice ffmpeg output. Hit enter to continue.");
            let _ = stdin().read_line(&mut String::new());
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    default_target: String,
    targets: Vec<Target>,
}

impl Config {
    fn find_target(&self, requested_target: &str) -> anyhow::Result<&Target> {
        self.targets
            .iter()
            .find(|t| t.name == requested_target)
            .ok_or_else(|| {
                anyhow!(
                    "could not find requested target \"{}\" in config",
                    requested_target
                )
            })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Target {
    name: String,
    format_spec: FormatSpec,
    default: DefaultFormat,
}

#[derive(Debug, Deserialize, Serialize)]
struct FormatSpec {
    audio: Formats,
    video: Formats,
    container: Formats,
    pix_fmt: Formats,
}

#[derive(Debug, Deserialize, Serialize)]
enum Formats {
    Allow(Vec<String>),
    Reject(Vec<String>),
}

#[derive(Debug, Deserialize, Serialize)]
struct DefaultFormat {
    audio: String,
    video: String,
    pix_fmt: String,
}
