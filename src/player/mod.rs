use api_models::player::*;
use api_models::playlist::{Category, DynamicPlaylistsPage, Playlists};
use api_models::state::{PlayerInfo, PlayingContext, SongProgress};

#[cfg(feature = "backend_lms")]
pub(crate) mod lms;
#[cfg(feature = "backend_mpd")]
pub(crate) mod mpd;

pub(crate) mod player_service;
pub(crate) mod spotify;
pub(crate) mod spotify_oauth;

pub trait Player {
    fn play(&mut self);
    fn pause(&mut self);
    fn next_track(&mut self);
    fn prev_track(&mut self);
    fn stop(&mut self);
    fn shutdown(&mut self);
    fn rewind(&mut self, seconds: i8);
    fn random_toggle(&mut self);
    fn load_playlist(&mut self, pl_id: String);
    fn load_album(&mut self, album_id: String);
    fn play_item(&mut self, id: String);
    fn remove_playlist_item(&mut self, id: String);
    fn get_song_progress(&mut self) -> SongProgress;
    fn get_current_song(&mut self) -> Option<Song>;
    fn get_player_info(&mut self) -> Option<PlayerInfo>;
    fn get_playing_context(&mut self, include_songs: bool) -> Option<PlayingContext>;
    fn get_playlist_categories(&mut self) -> Vec<Category>;
    fn get_static_playlists(&mut self) -> Playlists;
    fn get_dynamic_playlists(
        &mut self,
        category_ids: Vec<String>,
        offset: u32,
        limit: u32,
    ) -> Vec<DynamicPlaylistsPage>;
    fn get_playlist_items(&mut self, playlist_id: String) -> Vec<Song>;
}
