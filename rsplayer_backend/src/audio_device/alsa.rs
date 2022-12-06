use std::collections::HashMap;

use alsa::card;
use alsa::device_name::HintIter;
use alsa::mixer::{Selem, SelemChannelId};
use alsa::pcm::State;
use alsa::{Direction, Mixer};
use api_models::common::Volume;

use crate::common::Result;

use super::VolumeControlDevice;

#[allow(dead_code)]
const WAIT_TIME_MS: u64 = 10000;
#[allow(dead_code)]
const DELAY_MS: u64 = 100;

pub struct AlsaPcmCard {
    device_name: String,
}

impl AlsaPcmCard {
    #[allow(dead_code)]
    pub const fn new(device_name: String) -> Self {
        AlsaPcmCard { device_name }
    }
    #[allow(dead_code)]
    pub fn wait_unlock_audio_dev(&self) -> Result<()> {
        let mut elapsed_time: u64 = 0;
        while elapsed_time < WAIT_TIME_MS {
            if let Ok(dev) =
                alsa::PCM::new(self.device_name.as_str(), alsa::Direction::Playback, false)
            {
                let status = dev.status().unwrap();
                trace!(
                    "Device status {:?} after elapsed time {}",
                    &status.get_state(),
                    &elapsed_time
                );
                if status.get_state() != State::Running {
                    return Ok(());
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(DELAY_MS));
            elapsed_time += DELAY_MS;
        }
        Err(failure::format_err!(
            "Audio device [{}] remains locked after [{}]ms",
            self.device_name,
            &elapsed_time
        ))
    }

    #[allow(dead_code)]
    pub fn is_device_in_use(&self) -> bool {
        alsa::PCM::new(self.device_name.as_str(), alsa::Direction::Playback, false).is_err()
    }
    pub fn get_all_cards() -> HashMap<String, String> {
        let mut result = HashMap::new();
        for card in card::Iter::new().map(std::result::Result::unwrap) {
            result.insert(
                format!("hw:{}", card.get_index()),
                card.get_name().unwrap_or_default(),
            );
        }
        result
    }
}
const ALSA_MIXER_STEP: i64 = 1;
pub struct AlsaMixer {
    card_name: String,
}

impl AlsaMixer {
    pub fn new(card_name: String) -> Box<Self> {
        Box::new(AlsaMixer { card_name })
    }
}

impl VolumeControlDevice for AlsaMixer {
    fn vol_up(&self) -> Volume {
        let ev = self.get_vol();
        let nv = ev.current + ev.step;
        if nv <= ev.max {
            self.set_vol(nv)
        } else {
            ev
        }
    }

    fn vol_down(&self) -> Volume {
        let ev = self.get_vol();
        let nv = ev.current - ev.step;
        if nv >= ev.min {
            self.set_vol(nv)
        } else {
            ev
        }
    }

    fn get_vol(&self) -> Volume {
        if let Ok(mixer) = Mixer::new(self.card_name.as_str(), false) {
            if let Some(Some(selem)) = mixer.iter().next().map(Selem::new) {
                let (rmin, rmax) = selem.get_playback_volume_range();
                let mut channel = SelemChannelId::mono();
                for c in SelemChannelId::all().iter() {
                    if selem.has_playback_channel(*c) {
                        channel = *c;
                        break;
                    }
                }
                let old: i64 = selem.get_playback_volume(channel).unwrap();
                return Volume {
                    step: ALSA_MIXER_STEP,
                    min: rmin,
                    max: rmax,
                    current: old,
                };
            }
        }
        Volume::default()
    }

    fn set_vol(&self, level: i64) -> Volume {
        if let Ok(mixer) = Mixer::new(self.card_name.as_str(), false) {
            if let Some(Some(selem)) = mixer.iter().next().map(Selem::new) {
                let (rmin, rmax) = selem.get_playback_volume_range();
                for c in SelemChannelId::all().iter() {
                    if selem.has_playback_channel(*c) {
                        selem.set_playback_volume(*c, level).unwrap();
                    }
                }
                return Volume {
                    step: ALSA_MIXER_STEP,
                    min: rmin,
                    max: rmax,
                    current: level,
                };
            }
        }
        Volume::default()
    }
}
#[cfg(test)]
mod test {
    use std::ffi::CString;

    use alsa::{
        card,
        device_name::HintIter,
        mixer::{Selem, SelemChannelId, SelemId},
        HCtl, Mixer,
    };

    use crate::audio_device::VolumeControlDevice;

    use super::{AlsaMixer, AlsaPcmCard};

    #[test]

    fn test_set_volume() {
        for t in &["pcm", "ctl", "rawmidi", "timer", "seq", "hwdep"] {
            println!("{} devices:", t);
            let i = HintIter::new(None, &*CString::new(*t).unwrap()).unwrap();
            for a in i {
                if a.direction == None {
                    println!("  {:?}", a)

                }
            }
        }

        // let volume_ctrl = AlsaMixer::new("hw:0".to_string());

        // volume_ctrl.set_vol(80);
        // assert!(volume_ctrl.get_vol().current == 80);
    }

    #[test]
    fn print_mixer_of_cards() {
        for card in card::Iter::new().map(std::result::Result::unwrap) {
            println!(
                "[{}]:[{}]:[{}]",
                card.get_index(),
                card.get_name().unwrap(),
                card.get_longname().unwrap()
            );
            let mixer = Mixer::new(&format!("hw:{}", card.get_index()), false).unwrap();
            for selem in mixer.iter().filter_map(Selem::new) {
                let sid = selem.get_id();
                println!("\t{},{}:", sid.get_index(), sid.get_name().unwrap(),);

                if selem.has_volume() {
                    print!("\t  Volume limits: ");
                    if selem.has_capture_volume() {
                        let (vmin, vmax) = selem.get_capture_volume_range();
                        let (mbmin, mbmax) = selem.get_capture_db_range();
                        print!("Capture = {} - {}", vmin, vmax);
                        print!(" ({} dB - {} dB)", mbmin.to_db(), mbmax.to_db());
                    }
                    if selem.has_playback_volume() {
                        let (vmin, vmax) = selem.get_playback_volume_range();
                        let (mbmin, mbmax) = selem.get_playback_db_range();
                        print!("Playback = {} - {}", vmin, vmax);
                        print!(" ({} dB - {} dB)", mbmin.to_db(), mbmax.to_db());
                    }
                    println!();
                }

                if selem.is_enumerated() {
                    print!("\t  Valid values: ");
                    for v in selem.iter_enum().unwrap() {
                        print!("{}, ", v.unwrap());
                    }
                    print!("\n\t  Current values: ");
                    for v in SelemChannelId::all()
                        .iter()
                        .filter_map(|&v| selem.get_enum_item(v).ok())
                    {
                        print!("{}, ", selem.get_enum_item_name(v).unwrap());
                    }
                    println!();
                }

                if selem.can_capture() {
                    print!("\t  Capture channels: ");
                    for channel in SelemChannelId::all() {
                        if selem.has_capture_channel(*channel) {
                            print!("{}, ", channel);
                        };
                    }
                    println!();
                    print!("\t  Capture volumes: ");
                    for channel in SelemChannelId::all() {
                        if selem.has_capture_channel(*channel) {
                            print!(
                                "{}: {} ({} dB), ",
                                channel,
                                match selem.get_capture_volume(*channel) {
                                    Ok(v) => format!("{}", v),
                                    Err(_) => "n/a".to_string(),
                                },
                                match selem.get_capture_vol_db(*channel) {
                                    Ok(v) => format!("{}", v.to_db()),
                                    Err(_) => "n/a".to_string(),
                                }
                            );
                        }
                    }
                    println!();
                }

                if selem.can_playback() {
                    print!("\t  Playback channels: ");
                    if selem.is_playback_mono() {
                        print!("Mono");
                    } else {
                        for channel in SelemChannelId::all() {
                            if selem.has_playback_channel(*channel) {
                                print!("{}, ", channel);
                            };
                        }
                    }
                    println!();
                    if selem.has_playback_volume() {
                        print!("\t  Playback volumes: ");
                        for channel in SelemChannelId::all() {
                            if selem.has_playback_channel(*channel) {
                                print!(
                                    "{}: {} / {}dB, ",
                                    channel,
                                    match selem.get_playback_volume(*channel) {
                                        Ok(v) => format!("{}", v),
                                        Err(_) => "n/a".to_string(),
                                    },
                                    match selem.get_playback_vol_db(*channel) {
                                        Ok(v) => format!("{}", v.to_db()),
                                        Err(_) => "n/a".to_string(),
                                    }
                                );
                            }
                        }
                        println!();
                    }
                }
            }
        }
    }

    #[test]
    fn get_and_set_playback_volume() {
        let mixer = Mixer::new("hw:0", false).unwrap();
        let selem = mixer.find_selem(&SelemId::new("Master", 0)).unwrap();

        let (rmin, rmax) = selem.get_playback_volume_range();
        let mut channel = SelemChannelId::mono();
        for c in SelemChannelId::all().iter() {
            if selem.has_playback_channel(*c) {
                channel = *c;
                break;
            }
        }
        println!(
            "Testing on {} with limits {}-{} on channel {}",
            selem.get_id().get_name().unwrap(),
            rmin,
            rmax,
            channel
        );

        let old: i64 = selem.get_playback_volume(channel).unwrap();
        let new: i64 = rmax / 2;
        assert_ne!(new, old);

        println!("Changing volume of {} from {} to {}", channel, old, new);
        selem.set_playback_volume(channel, new).unwrap();
        let result: i64 = selem.get_playback_volume(channel).unwrap();
        assert_eq!(new, result);

        // return volume to old value
        // selem.set_playback_volume(channel, old).unwrap();
        // result = selem.get_playback_volume(channel).unwrap();
        // assert_eq!(old, result);
    }
    #[test]
    fn list_devices() {
        for h in HCtl::new("hw:0", false).expect("msg").elem_iter() {
            println!("DD :{:?}", h.get_id());
        }
    }
}

