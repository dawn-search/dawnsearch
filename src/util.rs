/*
   Copyright 2023 Krol Inventions B.V.

   This file is part of DawnSearch.

   DawnSearch is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   DawnSearch is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with DawnSearch.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use indicatif::{ProgressBar, ProgressStyle};

// See https://github.com/rust-lang/rfcs/issues/2566
pub fn slice_up_to(s: &str, max_len: usize) -> &str {
    if max_len >= s.len() {
        return s;
    }
    let mut idx = max_len;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

// From https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}

pub fn default_progress_bar(count: usize) -> ProgressBar {
    let progress = ProgressBar::new(count as u64);
    progress.set_style(
        ProgressStyle::with_template(
            "{prefix:>15} {elapsed_precise} [{bar:30}] {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    progress
}

/** Timestamp in seconds. */
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time travelled to before the epoch")
        .as_secs()
}
