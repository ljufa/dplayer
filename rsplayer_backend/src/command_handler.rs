use std::sync::Arc;

use api_models::common::PlayerCommand::{
    AddSongToQueue, LoadAlbum, LoadPlaylist, LoadSong, Next, Pause, Play, PlayItem, Prev,
    QueryCurrentPlayerInfo, QueryCurrentPlayingContext, QueryCurrentSong,
    QueryCurrentStreamerState, QueryDynamicPlaylists, QueryPlaylistItems, RandomToggle,
    RemovePlaylistItem, Rewind,
};
use api_models::common::SystemCommand::{ChangeAudioOutput, PowerOff, SetVol, VolDown, VolUp};
use api_models::common::{PlayerCommand, SystemCommand};
use api_models::state::StateChangeEvent;

use log::debug;
use rsplayer_config::MutArcConfiguration;
use rsplayer_metadata::metadata::MetadataService;
use rsplayer_playback::player_service::ArcPlayerService;
use tokio::sync::broadcast::Sender;

use rsplayer_hardware::audio_device::audio_service::ArcAudioInterfaceSvc;

#[allow(clippy::too_many_lines)]
pub async fn handle_player_commands(
    player_service: ArcPlayerService,
    config_store: MutArcConfiguration,
    mut input_commands_rx: tokio::sync::mpsc::Receiver<PlayerCommand>,
    state_changes_sender: Sender<StateChangeEvent>,
) -> ! {
    loop {
        let Some(cmd ) = input_commands_rx.recv().await else {
            continue;
        };
        debug!("Received command {:?}", cmd);
        match cmd {
            Play => {
                player_service
                    .get_current_player()
                    .play_from_current_queue_song();
            }
            PlayItem(id) => {
                player_service.get_current_player().play_song(&id);
            }
            RemovePlaylistItem(id) => {
                player_service
                    .get_current_player()
                    .remove_song_from_queue(&id);
            }
            Pause => {
                player_service.get_current_player().pause_current_song();
            }
            Next => {
                player_service.get_current_player().play_next_song();
            }
            Prev => {
                player_service.get_current_player().play_prev_song();
            }
            Rewind(sec) => {
                player_service.get_current_player().seek_current_song(sec);
            }
            LoadPlaylist(pl_id) => {
                player_service
                    .get_current_player()
                    .load_playlist_in_queue(&pl_id);
            }
            LoadAlbum(album_id) => {
                player_service
                    .get_current_player()
                    .load_album_in_queue(&album_id);
            }
            RandomToggle => player_service.get_current_player().toggle_random_play(),

            LoadSong(song_id) => player_service
                .get_current_player()
                .load_song_in_queue(&song_id),
            AddSongToQueue(song_id) => player_service
                .get_current_player()
                .add_song_in_queue(&song_id),
            PlayerCommand::ClearQueue => {
                player_service.get_current_player().clear_queue();
            }
            PlayerCommand::SaveQueueAsPlaylist(playlist_name) => {
                player_service
                    .get_current_player()
                    .save_queue_as_playlist(&playlist_name);
            }
            /*
             * Query commands
             */
            QueryCurrentSong => {
                if let Some(song) = player_service.get_current_player().get_current_song() {
                    state_changes_sender
                        .send(StateChangeEvent::CurrentSongEvent(song))
                        .unwrap();
                }
            }
            QueryCurrentPlayingContext(query) => {
                if let Some(pc) = player_service
                    .get_current_player()
                    .get_playing_context(query)
                {
                    state_changes_sender
                        .send(StateChangeEvent::CurrentPlayingContextEvent(pc))
                        .unwrap();
                }
            }
            QueryCurrentPlayerInfo => {
                if let Some(pi) = player_service.get_current_player().get_player_info() {
                    state_changes_sender
                        .send(StateChangeEvent::PlayerInfoEvent(pi))
                        .unwrap();
                }
            }
            QueryCurrentStreamerState => {
                let ss = config_store.lock().unwrap().get_streamer_status();
                state_changes_sender
                    .send(StateChangeEvent::StreamerStateEvent(ss))
                    .unwrap();
            }
            QueryDynamicPlaylists(category_ids, offset, limit) => {
                let dynamic_pls = player_service.get_current_player().get_dynamic_playlists(
                    category_ids,
                    offset,
                    limit,
                );
                state_changes_sender
                    .send(StateChangeEvent::DynamicPlaylistsPageEvent(dynamic_pls))
                    .unwrap();
            }
            QueryPlaylistItems(playlist_id, page_no) => {
                let pl_items = player_service
                    .get_current_player()
                    .get_playlist_items(&playlist_id, page_no);
                state_changes_sender
                    .send(StateChangeEvent::PlaylistItemsEvent(pl_items, page_no))
                    .unwrap();
            }
        }
    }
}
pub async fn handle_system_commands(
    ai_service: ArcAudioInterfaceSvc,
    metadata_service: Arc<MetadataService>,
    config_store: MutArcConfiguration,
    mut input_commands_rx: tokio::sync::mpsc::Receiver<SystemCommand>,
    state_changes_sender: Sender<StateChangeEvent>,
) {
    loop {
        if let Some(cmd) = input_commands_rx.recv().await {
            debug!("Received command {:?}", cmd);
            match cmd {
                SetVol(val) => {
                    let nv = ai_service.set_volume(i64::from(val));
                    let new_state = config_store.lock().unwrap().save_volume_state(nv);
                    state_changes_sender
                        .send(StateChangeEvent::StreamerStateEvent(new_state))
                        .expect("Send event failed.");
                }
                VolUp => {
                    let nv = ai_service.volume_up();
                    let new_state = config_store.lock().unwrap().save_volume_state(nv);
                    state_changes_sender
                        .send(StateChangeEvent::StreamerStateEvent(new_state))
                        .expect("Send event failed.");
                }
                VolDown => {
                    let nv = ai_service.volume_down();
                    let new_state = config_store.lock().unwrap().save_volume_state(nv);
                    state_changes_sender
                        .send(StateChangeEvent::StreamerStateEvent(new_state))
                        .expect("Send event failed.");
                }
                ChangeAudioOutput => {
                    if let Some(out) = ai_service.toggle_output() {
                        let new_state = config_store.lock().unwrap().save_audio_output(out);
                        state_changes_sender
                            .send(StateChangeEvent::StreamerStateEvent(new_state))
                            .unwrap();
                    };
                }
                PowerOff => {
                    std::process::Command::new("/sbin/poweroff")
                        .spawn()
                        .expect("halt command failed");
                }
                SystemCommand::RestartSystem => {
                    std::process::Command::new("/sbin/poweroff")
                        .arg("--reboot")
                        .spawn()
                        .expect("halt command failed");
                }
                SystemCommand::RestartRSPlayer => {
                    std::process::Command::new("systemctl")
                        .arg("restart")
                        .arg("rsplayer")
                        .spawn()
                        .expect("Failed to restart rsplayer service");
                }
                SystemCommand::RescanMetadata(music_dir) => {
                    let mtds = metadata_service.clone();
                    std::thread::Builder::new()
                        .name("metadata_scanner".to_string())
                        .spawn(move || mtds.scan_music_dir(music_dir, true))
                        .expect("Failed to start metadata scanner thread");
                }
            }
        }
    }
}