use std::time::{SystemTime, UNIX_EPOCH};

pub fn local_time(at: SystemTime) -> String {
    let tm = local_tm(at);
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}

pub fn local_date_time(at: SystemTime) -> String {
    let tm = local_tm(at);
    format!(
        "{}-{:02}-{:02} {}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        local_time(at)
    )
}

fn local_tm(at: SystemTime) -> libc::tm {
    let seconds = at
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as libc::time_t);
    // SAFETY: localtime_r writes only into `tm`, which outlives the call; both pointers are valid.
    unsafe {
        let mut tm = std::mem::zeroed::<libc::tm>();
        libc::localtime_r(&seconds, &mut tm);
        tm
    }
}
