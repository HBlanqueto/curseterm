use std::time::{SystemTime, UNIX_EPOCH};

pub fn local_time(at: SystemTime) -> String {
    let seconds = at
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as libc::time_t);
    // SAFETY: localtime_r writes only into `tm`, which outlives the call; both pointers are valid.
    let tm = unsafe {
        let mut tm = std::mem::zeroed::<libc::tm>();
        libc::localtime_r(&seconds, &mut tm);
        tm
    };
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}
