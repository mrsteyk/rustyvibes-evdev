use std::os::fd::AsRawFd;

use bpaf::{construct, long, positional, Parser};

use evdev::{Device, EventType, InputEventKind};
use mio::{unix::SourceFd, Events, Interest, Poll, Token};

#[derive(Clone, Debug)]
struct Args {
    volume: u16,
    soundpack: String,
}

mod sound;

fn main() {
    let volume = long("volume")
        .short('v')
        .help("Volume")
        .argument("VOLUME")
        .fallback(100)
        .display_fallback();
    let soundpack = positional("SOUNDPACK");

    let parser = construct!(Args { volume, soundpack });
    let args = parser.to_options().descr("mechanical sound stuff idk");
    let args = args.run();
    println!("{:?} wow pipebomb! So cool!", args);

    unsafe {
        libc::nice(-20);
    }

    // https://github.com/LiveSplit/livesplit-core/blob/b3733b0350c63580c9a0438a34f8eed0c05fd079/crates/livesplit-hotkey/src/linux/evdev_impl.rs
    // Low numbered tokens are allocated to devices.
    // const PING_TOKEN: Token = Token(usize::MAX);
    let mut poll = Poll::new().unwrap();
    // let waker = Waker::new(poll.registry(), PING_TOKEN).unwrap();

    let mut devices: Vec<Device> = evdev::enumerate()
        .map(|(_, d)| d)
        .filter(|d| d.supported_events().contains(EventType::KEY))
        .collect();

    for (i, fd) in devices.iter().enumerate() {
        poll.registry()
            .register(&mut SourceFd(&fd.as_raw_fd()), Token(i), Interest::READABLE)
            .unwrap();
    }

    let mut events = Events::with_capacity(1024);

    type JSONInner = serde_json::Map<String, serde_json::Value>;
    let path = std::path::Path::new(&args.soundpack);
    let config = std::fs::read_to_string(path.join("config.json"))
        .expect("failed to read soundpack config!");
    let config = serde_json::from_str::<JSONInner>(&config).unwrap();
    let default_key_path = config["defines"]["30"]
        .as_str()
        .expect("unable to get sound for 30 scancode (default)!");

    println!("started!");

    'event_loop: loop {
        if poll.poll(&mut events, None).is_err() {
            eprintln!("epoll err!");
            break 'event_loop;
        }

        for mio_event in &events {
            if mio_event.token().0 < devices.len() {
                let idx = mio_event.token().0;
                for ev in devices[idx].fetch_events().unwrap() {
                    if let InputEventKind::Key(k) = ev.kind() {
                        const RELEASED: i32 = 0;
                        const PRESSED: i32 = 1;
                        match ev.value() {
                            PRESSED => {
                                // println!("{:?} {}", k, k.0 as u32);
                                let filename = config["defines"][k.0.to_string()]
                                    .as_str()
                                    .unwrap_or(default_key_path);
                                sound::play_sound(
                                    path.join(filename).to_str().unwrap(),
                                    args.volume,
                                );
                            }
                            RELEASED => {
                                // Maybe someday release sound
                            }
                            _ => {} // Ignore repeating
                        }
                    }
                }
            }
        }
    }
}
