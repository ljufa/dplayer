use std::sync::{
    atomic::{AtomicBool, AtomicU16, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;

use log::{debug, error, info, warn};
use sled::Db;
use thread_priority::{ThreadBuilder, ThreadPriority};
use tokio::sync::broadcast::Sender;

use api_models::{
    settings::{RsPlayerSettings, Settings},
    state::{PlayerState, StateChangeEvent},
};
use rsplayer_metadata::metadata_service::MetadataService;
use rsplayer_metadata::queue_service::QueueService;

use super::symphonia::PlaybackResult;

pub struct PlayerService {
    state_db: Db,
    queue_service: Arc<QueueService>,
    #[allow(dead_code)]
    metadata_service: Arc<MetadataService>,
    playback_thread_handle: Arc<Mutex<Option<JoinHandle<PlaybackResult>>>>,
    stop_signal: Arc<AtomicBool>,
    skip_to_time: Arc<AtomicU16>,
    audio_device: String,
    rsp_settings: RsPlayerSettings,
    music_dir: String,
    changes_tx: Sender<StateChangeEvent>,
}
const LAST_SONG_PAUSED_KEY: &str = "last_song_paused";
const LAST_SONG_PROGRESS_KEY: &str = "last_played_song_progress";

impl PlayerService {
    #[must_use]
    pub fn new(
        settings: &Settings,
        metadata_service: Arc<MetadataService>,
        queue_service: Arc<QueueService>,
        state_changes_tx: Sender<StateChangeEvent>,
    ) -> Self {
        let db = sled::open("player_state").expect("Failed to open queue db");
        let state_db = db.clone();
        let mut rx = state_changes_tx.subscribe();
        tokio::task::spawn(async move {
            let mut i = 0;
            loop {
                match rx.recv().await {
                    Ok(StateChangeEvent::SongTimeEvent(st)) => {
                        i += 1;
                        if i % 2 == 0 {
                            let lt = st.current_time.as_secs().to_string();
                            debug!("Save time state: {lt}");
                            _ = state_db.insert(LAST_SONG_PROGRESS_KEY, lt.as_bytes());
                        }
                    }
                    Ok(StateChangeEvent::PlaybackStateEvent(ps)) => {
                        debug!("Save player state: {:?}", ps);
                        _ = match ps {
                            PlayerState::PLAYING => state_db.remove(LAST_SONG_PAUSED_KEY),
                            PlayerState::PAUSED | PlayerState::STOPPED => state_db.insert(LAST_SONG_PAUSED_KEY, "true"),
                        };
                    }
                    _ => (),
                }
            }
        });
        let ps = PlayerService {
            state_db: db,
            changes_tx: state_changes_tx,
            queue_service,
            metadata_service,
            playback_thread_handle: Arc::new(Mutex::new(None)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            skip_to_time: Arc::new(AtomicU16::new(0)),
            audio_device: settings.alsa_settings.output_device.name.clone(),
            rsp_settings: settings.rs_player_settings.clone(),
            music_dir: settings.metadata_settings.music_directory.clone(),
        };
        let last_played_song_progress = ps.get_last_played_song_time();
        if last_played_song_progress > 0 {
            ps.seek_current_song(last_played_song_progress);
        }
        ps
    }

    pub fn play_from_current_queue_song(&self) {
        if self.is_playing() {
            self.changes_tx
                .send(StateChangeEvent::PlaybackStateEvent(PlayerState::PLAYING))
                .ok();
            return;
        }
        if let Some(s) = self.queue_service.get_current_song() {
            self.metadata_service.increase_play_count(&s.file);
        }
        if let Ok(Some(_)) = self.state_db.get(LAST_SONG_PAUSED_KEY) {
            let last_song_time = self.get_last_played_song_time();
            self.seek_current_song(last_song_time);
        }

        *self.playback_thread_handle.lock().unwrap() = Some(self.play_all_in_queue());
    }

    pub fn play_next_song(&self) {
        self.stop_current_song();
        self.queue_service.move_current_to_next_song();
        self.play_from_current_queue_song();
    }

    pub fn play_prev_song(&self) {
        self.stop_current_song();
        self.queue_service.move_current_to_previous_song();
        self.play_from_current_queue_song();
    }

    pub fn stop_current_song(&self) {
        self.stop_signal.store(false, Ordering::Relaxed);
        self.await_playing_song_to_finish();
    }

    pub fn pause_current_song(&self) {
        if self.is_playing() {
            self.stop_current_song();
        }
    }

    #[allow(clippy::unused_self, clippy::missing_const_for_fn)]
    pub fn seek_current_song(&self, seconds: u16) {
        self.skip_to_time.store(seconds, Ordering::Relaxed);
    }

    pub fn play_song(&self, song_id: &str) {
        self.stop_current_song();
        self.queue_service.move_current_to(song_id);
        self.play_from_current_queue_song();
    }

    fn await_playing_song_to_finish(&self) {
        while self.is_playing() {
            continue;
        }
        debug!("aWait finished");
    }
    #[allow(clippy::significant_drop_tightening)]
    fn is_playing(&self) -> bool {
        let binding = self.playback_thread_handle.clone();
        let mg = binding.lock().unwrap();
        let handle = mg.as_ref();
        handle.map_or(false, |f| {
            let finished = f.is_finished();
            !finished
        })
    }

    fn play_all_in_queue(&self) -> JoinHandle<PlaybackResult> {
        self.stop_signal.store(true, Ordering::Relaxed);
        let stop_signal = self.stop_signal.clone();
        let skip_to_time = self.skip_to_time.clone();
        let queue = self.queue_service.clone();
        let audio_device = self.audio_device.clone();
        let playback_thread_prio = self.rsp_settings.player_threads_priority;
        let music_dir = self.music_dir.clone();
        let queue_size = queue.get_all_songs().len();
        let changes_tx = self.changes_tx.clone();
        let rsp_settings = self.rsp_settings.clone();
        let is_multi_core_platform = core_affinity::get_core_ids().map_or(false, |ids| ids.len() > 1);
        let prio = if is_multi_core_platform {
            ThreadPriority::Crossplatform(playback_thread_prio.try_into().unwrap())
        } else {
            ThreadPriority::Min
        };
        ThreadBuilder::default()
            .name("playback".to_string())
            .priority(prio)
            .spawn(move |prio| {
                if prio.is_ok() {
                    info!("Playback thread started with priority {:?}", playback_thread_prio);
                } else {
                    warn!("Failed to set playback thread priority");
                }
                if is_multi_core_platform {
                    if let Some(Some(last_core)) = core_affinity::get_core_ids().map(|ids| ids.last().copied()) {
                        if core_affinity::set_for_current(last_core) {
                            info!("Playback thread set to last core {:?}", last_core);
                        } else {
                            warn!("Failed to set playback thread to last core {:?}", last_core);
                        }
                    }
                }
                let mut num_failed = 0;

                loop {
                    let Some(song) = queue.get_current_song() else {
                        stop_signal.store(false, Ordering::Relaxed);
                        changes_tx
                            .send(StateChangeEvent::PlaybackStateEvent(PlayerState::STOPPED))
                            .ok();
                        break PlaybackResult::QueueFinished;
                    };
                    changes_tx
                        .send(StateChangeEvent::CurrentSongEvent(song.clone()))
                        .expect("msg send failed");
                    changes_tx
                        .send(StateChangeEvent::PlaybackStateEvent(PlayerState::PLAYING))
                        .expect("msg send failed");
                    match super::symphonia::play_file(
                        &song.file,
                        &stop_signal,
                        &skip_to_time,
                        &audio_device,
                        &rsp_settings,
                        &music_dir,
                        &changes_tx,
                    ) {
                        Ok(PlaybackResult::PlaybackStopped) => {
                            stop_signal.store(false, Ordering::Relaxed);
                            changes_tx
                                .send(StateChangeEvent::PlaybackStateEvent(PlayerState::STOPPED))
                                .ok();
                            break PlaybackResult::PlaybackStopped;
                        }
                        Err(err) => {
                            error!("Failed to play file {}. Error: {:?}", song.file, err);
                            num_failed += 1;
                            if song.file.starts_with("http") || num_failed == 10 || num_failed >= queue_size {
                                warn!("Number of failed songs is greater than 10. Aborting.");
                                stop_signal.store(false, Ordering::Relaxed);
                                changes_tx
                                    .send(StateChangeEvent::PlaybackStateEvent(PlayerState::STOPPED))
                                    .ok();
                                break PlaybackResult::QueueFinished;
                            }
                        }
                        res => {
                            info!("Playback finished with result {:?}", res);
                            num_failed = 0;
                        }
                    }

                    if !queue.move_current_to_next_song() {
                        break PlaybackResult::QueueFinished;
                    }
                }
            })
            .unwrap()
    }

    fn get_last_played_song_time(&self) -> u16 {
        let last_time = match self.state_db.get(LAST_SONG_PROGRESS_KEY) {
            Ok(Some(lt)) => {
                let v = lt.to_vec();
                String::from_utf8(v).unwrap()
            }
            _ => "0".to_string(),
        };
        last_time.parse::<u16>().unwrap_or_default()
    }
}
