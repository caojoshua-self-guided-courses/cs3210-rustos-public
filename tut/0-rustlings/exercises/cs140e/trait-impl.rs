// FIXME: Make me pass! Diff budget: 25 lines.

#[derive(Debug)]
enum Duration {
    MilliSeconds(u64),
    Seconds(u32),
    Minutes(u16)
}

use Duration::MilliSeconds;
use Duration::Seconds;
use Duration::Minutes;

impl std::cmp::PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        toMilliSeconds(self) == toMilliSeconds(other)
    }
}

fn toMilliSeconds(dur: &Duration) -> u64 {
    match dur {
        MilliSeconds(milliSeconds) => *milliSeconds,
        Seconds(seconds) => 1000 * *seconds as u64,
        Minutes(minutes) => 60 * 1000 * *minutes as u64
    }
}

#[test]
fn traits() {
    assert_eq!(Seconds(120), Minutes(2));
    assert_eq!(Seconds(420), Minutes(7));
    assert_eq!(MilliSeconds(420000), Minutes(7));
    assert_eq!(MilliSeconds(43000), Seconds(43));
}
