#[derive(Debug)]
pub(crate) struct AudioPlayer;

qi::object! {
    impl AudioPlayer {

        fn play_sound(info: SoundInfo) -> Sound {
            Sound {
                info: SoundInfo,
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Buffer {
    data: bytes::Bytes,
}

qi::object! {
    impl Buffer {
        fn seek(&mut self, position: usize) {
        }

        fn position(&self) -> usize {

        }

        fn read_bytes(&mut self, size: usize) -> Bytes {
        }
    }
}

#[derive(Debug)]
pub(crate) struct Sound {
    buffer: Buffer,
}

qi::object! {
    impl Sound {
        fn pause(&mut self) {
        }

        fn resume(&mut self) {}

        fn is_playing(&self) -> bool {
        }

        fn info() {
        }
    }
}

#[derive(Debug, qi::Valuable)]
pub(crate) struct SoundInfo {
    name: String,
    data: Buffer,
}
