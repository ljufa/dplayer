use std::borrow::BorrowMut;
use std::time::Duration;

use api_models::player::*;
use api_models::playlist::Playlist;
use api_models::settings::*;
use mpd::Client;
use num_traits::ToPrimitive;

use crate::common::Result;

use super::Player;

pub struct MpdPlayerClient {
    mpd_client: Client,
    mpd_server_url: String,
}

unsafe impl Send for MpdPlayerClient {}

impl MpdPlayerClient {
    pub fn new(mpd_settings: &MpdSettings) -> Result<MpdPlayerClient> {
        if !mpd_settings.enabled {
            return Err(failure::err_msg("MPD player integration is disabled."));
        }
        Ok(MpdPlayerClient {
            mpd_client: create_client(mpd_settings)?,
            mpd_server_url: mpd_settings.get_server_url(),
        })
    }
    fn try_with_reconnect<F>(
        &mut self,
        command_event: StatusChangeEvent,
        command: F,
    ) -> Result<StatusChangeEvent>
    where
        F: FnMut(&mut Client) -> core::result::Result<(), mpd::error::Error>,
    {
        match self.try_with_reconnect_result(command) {
            Ok(_) => Ok(command_event),
            Err(e) => Err(e),
        }
    }

    fn try_with_reconnect_result<F, R>(&mut self, mut command: F) -> Result<R>
    where
        F: FnMut(&mut Client) -> mpd::error::Result<R>,
    {
        let mut result = command(self.mpd_client.borrow_mut());
        if result.is_err() {
            match Client::connect(self.mpd_server_url.as_str()) {
                Ok(cl) => {
                    self.mpd_client = cl;
                    result = command(self.mpd_client.borrow_mut());
                }
                Err(e) => result = Err(e),
            }
        }
        match result {
            Ok(r) => Ok(r),
            Err(e) => Err(failure::format_err!("{}", e)),
        }
    }
}

impl Player for MpdPlayerClient {
    fn play(&mut self) -> Result<StatusChangeEvent> {
        self.try_with_reconnect(StatusChangeEvent::Playing, |client| client.play())
    }

    fn pause(&mut self) -> Result<StatusChangeEvent> {
        self.try_with_reconnect(StatusChangeEvent::Paused, |client| client.pause(true))
    }

    fn next_track(&mut self) -> Result<StatusChangeEvent> {
        self.try_with_reconnect(StatusChangeEvent::SwitchedToNextTrack, |client| {
            client.next()
        })
    }

    fn prev_track(&mut self) -> Result<StatusChangeEvent> {
        self.try_with_reconnect(StatusChangeEvent::SwitchedToPrevTrack, |client| {
            client.prev()
        })
    }

    fn stop(&mut self) -> Result<StatusChangeEvent> {
        self.try_with_reconnect(StatusChangeEvent::Stopped, |client| client.stop())
    }

    fn shutdown(&mut self) {
        info!("Shutting down MPD player!");
        _ = self.stop();
        _ = self.mpd_client.close();
        info!("MPD player shutdown finished!");
    }

    fn rewind(&mut self, seconds: i8) -> Result<StatusChangeEvent> {
        let result = self.try_with_reconnect_result(|client| client.status());
        if let Ok(status) = result {
            //todo: implement protection against going of the range
            let position = status.elapsed.unwrap().num_seconds() + seconds as i64;
            self.mpd_client.rewind(position)?;
        }
        Ok(StatusChangeEvent::Playing)
    }

    fn get_current_track_info(&mut self) -> Option<CurrentTrackInfo> {
        let result = self.try_with_reconnect_result(|client| client.currentsong());
        let song = result.unwrap_or(None);
        if song.is_none() {
            warn!("Mpd Song is None");
            return None;
        }
        let song = song.unwrap();
        trace!("Song is {:?}", song);

        let mut album: Option<String> = None;
        if song.tags.contains_key("Album") {
            album = Some(song.tags["Album"].clone());
        }
        let mut artist: Option<String> = None;
        if song.tags.contains_key("Artist") {
            artist = Some(song.tags["Artist"].clone());
        }
        let mut genre: Option<String> = None;
        if song.tags.contains_key("Genre") {
            genre = Some(song.tags["Genre"].clone());
        }
        let mut date: Option<String> = None;
        if song.tags.contains_key("Date") {
            date = Some(song.tags["Date"].clone());
        }
        Some(CurrentTrackInfo {
            filename: Some(song.file),
            name: song.name,
            album,
            artist,
            genre,
            date,
            title: song.title,
            uri: None,
        })
    }

    fn get_player_info(&mut self) -> Option<PlayerInfo> {
        let status = self.try_with_reconnect_result(|client| client.status());
        trace!("Mpd Status is {:?}", status);
        if let Ok(status) = status {
            Some(PlayerInfo {
                audio_format_bit: status.audio.map(|f| f.bits),
                audio_format_rate: status.audio.map(|f| f.rate),
                audio_format_channels: status.audio.map(|f| f.chans as u32),
                random: Some(status.random),
                state: Some(convert_state(status.state)),
                time: status.time.map_or((Duration::ZERO, Duration::ZERO), |t| {
                    (
                        Duration::from_nanos(
                            t.0.num_nanoseconds()
                                .unwrap_or_default()
                                .to_u64()
                                .unwrap_or_default(),
                        ),
                        Duration::from_nanos(
                            t.1.num_nanoseconds()
                                .unwrap_or_default()
                                .to_u64()
                                .unwrap_or_default(),
                        ),
                    )
                }),
            })
        } else {
            error!("Error while getting mpd status {:?}", status);
            None
        }
    }

    fn random_toggle(&mut self) {
        let status = self.try_with_reconnect_result(|client| client.status());
        if let Ok(status) = status {
            _ = self.mpd_client.random(!status.random);
        }
    }

    fn get_playlists(&mut self) -> Vec<Playlist> {
        let pls = self
            .try_with_reconnect_result(|client| client.playlists())
            .unwrap_or_default();
        pls.into_iter().map(|p| Playlist { name: p.name }).collect()
    }

    fn load_playlist(&mut self, pl_name: String) {
        let r = self.try_with_reconnect(
            StatusChangeEvent::PlaylistLoaded(pl_name.clone()),
            |client| {
                _ = client.clear();
                client.load(pl_name.clone(), ..)
            },
        );
        info!("Load pl result: {:?}", r);
    }
}
fn convert_state(mpd_state: mpd::status::State) -> PlayerState {
    match mpd_state {
        mpd::State::Stop => PlayerState::STOPPED,
        mpd::State::Play => PlayerState::PLAYING,
        mpd::State::Pause => PlayerState::PAUSED,
    }
}
impl Drop for MpdPlayerClient {
    fn drop(&mut self) {
        self.shutdown()
    }
}
fn create_client(mpd_settings: &MpdSettings) -> Result<Client> {
    let mut tries = 0;
    let mut connection = None;
    while tries < 5 {
        tries += 1;
        info!(
            "Trying to connect to MPD server {}. Attempt no: {}",
            mpd_settings.get_server_url(),
            tries,
        );
        let conn = Client::connect(mpd_settings.get_server_url().as_str());
        match conn {
            Ok(conn) => {
                info!("Mpd client created");
                connection = Some(conn);
                break;
            }
            Err(e) => {
                error!("Failed to connect to mpd server {}", e);
                std::thread::sleep(Duration::from_secs(1))
            }
        }
    }
    match connection {
        Some(c) => Ok(c),
        None => Err(failure::err_msg("Can't connecto to MPD server!")),
    }
}
