use std::{
    fs::File,
    time::{self, Duration},
};

use anyhow::Result;
use api_models::{common::to_database_key, player::Song, settings::MetadataStoreSettings};
use log::{info, warn};
use sled::Db;
use symphonia::core::{
    formats::FormatOptions,
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::{MetadataOptions, StandardTagKey, Tag},
    probe::{Hint, ProbeResult},
};
use walkdir::WalkDir;

use mockall::automock;

pub struct MetadataService {
    db: Db,
    settings: MetadataStoreSettings,
}

#[automock]
impl MetadataService {
    pub fn new(settings: &MetadataStoreSettings) -> Result<Self> {
        let settings = settings.clone();
        let db = sled::open(settings.db_path.as_str())?;
        Ok(Self { db, settings })
    }

    pub fn get_song(&self, song_id: &str) -> Option<Song> {
        if let Ok(Some(song)) = self.db.get(song_id) {
            Song::bytes_to_song(&song)
        } else {
            None
        }
    }

    pub fn scan_music_dir(&self, music_dir: String, full_scan: bool) {
        if full_scan {
            _ = self.db.clear();
        }
        let start_time = time::Instant::now();
        let supported_ext = &self.settings.supported_extensions;
        let mut count = 0;
        for entry in WalkDir::new(music_dir)
            .follow_links(self.settings.follow_links)
            .sort_by_file_name()
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|de| de.file_type().is_file())
            .filter(|de| {
                let ext = de.path().extension().map_or("no_ext".to_string(), |ex| {
                    ex.to_str().unwrap().to_lowercase()
                });
                supported_ext.contains(&ext)
            })
        {
            let file_path = entry.path();
            let file = Box::new(File::open(file_path).unwrap());
            let mss = MediaSourceStream::new(file, MediaSourceStreamOptions::default());

            let mut hint = Hint::new();
            if let Some(ext) = file_path.extension() {
                let ext = ext.to_str().unwrap().to_lowercase();
                hint.with_extension(&ext);
            }
            let format_opts = FormatOptions {
                enable_gapless: false,
                ..Default::default()
            };
            // Use the default options for metadata readers.
            let metadata_opts = MetadataOptions::default();
            let file_p = file_path.to_str().unwrap();
            info!("Scanning file:\t{}", file_p);
            match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
                Ok(mut probed) => {
                    let mut song = build_song(&mut probed);
                    let db_key = to_database_key(file_p);
                    song.id = db_key.clone();
                    song.file = file_p.to_string();
                    log::debug!("Add/update song in database: {:?}", song);
                    _ = self.db.insert(&db_key, song.to_json_string_bytes());
                    count += 1;
                }
                Err(err) => warn!("Error:{} {}", file_p, err),
            }
        }
        _ = self.db.flush();
        info!(
            "Scanning of {} files finished in {}s",
            count,
            start_time.elapsed().as_secs()
        );
    }

    pub fn get_all_songs_iterator(&self) -> impl Iterator<Item = Song> {
        self.db
            .iter()
            .filter_map(std::result::Result::ok)
            .map_while(|s| Song::bytes_to_song(&s.1))
    }
}

fn build_song(probed: &mut ProbeResult) -> Song {
    let mut song = Song::default();
    if let Some(track) = probed.format.default_track() {
        let params = &track.codec_params;
        if let Some(n_frames) = params.n_frames {
            if let Some(tb) = params.time_base {
                let time = tb.calc_time(n_frames);
                song.time = Some(Duration::from_secs(time.seconds));
            }
        }
    }
    if let Some(metadata_rev) = probed.format.metadata().current() {
        let tags = metadata_rev.tags();
        for known_tag in tags.iter().filter(|t| t.is_known()) {
            match known_tag.std_key.unwrap_or(StandardTagKey::Version) {
                StandardTagKey::Album => song.album = from_tag_value_to_option(known_tag),
                StandardTagKey::AlbumArtist => {
                    song.album_artist = from_tag_value_to_option(known_tag);
                }
                StandardTagKey::Artist => song.artist = from_tag_value_to_option(known_tag),
                StandardTagKey::Composer => song.composer = from_tag_value_to_option(known_tag),
                StandardTagKey::Date => song.date = from_tag_value_to_option(known_tag),
                StandardTagKey::DiscNumber => song.disc = from_tag_value_to_option(known_tag),
                StandardTagKey::Genre => song.genre = from_tag_value_to_option(known_tag),
                StandardTagKey::Label => song.label = from_tag_value_to_option(known_tag),
                StandardTagKey::Performer => song.performer = from_tag_value_to_option(known_tag),
                StandardTagKey::TrackNumber => song.track = from_tag_value_to_option(known_tag),
                StandardTagKey::TrackTitle => {
                    let title = from_tag_value_to_option(known_tag);
                    song.title = title;
                }
                _ => {}
            }
        }
        for unknown_tag in tags.iter().filter(|t| !t.is_known()) {
            song.tags.insert(
                unknown_tag.key.clone(),
                from_tag_value_to_option(unknown_tag).unwrap_or_default(),
            );
        }
    }
    song
}

#[allow(clippy::unnecessary_wraps)]
fn from_tag_value_to_option(tag: &Tag) -> Option<String> {
    Some(tag.value.to_string())
}
