// First attempt. Only reads the file and then computes the minimum across all towns.
// Basic idea is to test different strategies for reading and processing. Main holds the
// more polished code.

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::time::Instant;
use std::thread::{scope};

// Laptop only has 14 cores.
const THREADS: usize = 14;

struct TempBuff<'a>{
    leftover: Vec<u8>,
    buf: &'a [u8]
}

fn main() -> io::Result<()> {
    let start = Instant::now();
    let mut count = 0;

    // Scan file and calculate minimum temperature.
    let file = File::open("measurements.txt")?;
    let reader = io::BufReader::with_capacity(1000 * 1000 * 1024, file);

    // let min = calc_min1(reader)?;
    // let min = calc_min2(reader, &mut count)?;
    let min = calc_min3(reader, &mut count)?;

    println!("Minimum Value: {}", min);
    let duration = start.elapsed();   // <-- stop timer
    println!("Runtime: {:?}", duration);
    Ok(())
}

// No Strings. Multithreaded.
pub fn calc_min3(mut reader: BufReader<File>, count: &mut u64) -> io::Result<f64>{
    let mut min = 100000.0;

    let mut leftover = Vec::<u8>::new();

    loop {
        *count += 1;
        let buf = reader.fill_buf()?;

        if buf.is_empty() & leftover.is_empty(){
            break;
        }

        // Combine leftover and buf.
        let mut combined = TempBuff{leftover: leftover, buf: buf};

        // Find first newline in buf.
        let split_at_l = combined
            .buf
            .iter()
            .position(|&b| b == b'\n')
            .map(|idx| idx + 1)      // include newline
            .unwrap_or(0);           // no newline → nothing processable

        // Find last newline in buf.
        let split_at_r = combined
            .buf
            .iter()
            .rposition(|&b| b == b'\n')
            .map(|idx| idx + 1)      // include newline
            .unwrap_or(0);           // no newline → nothing processable

        // Get partial line from buf.
        leftover = Vec::new();
        leftover.extend_from_slice(&buf[split_at_r..]);

        // Trim partial line from buf.
        combined.leftover.extend_from_slice(&buf[..split_at_l]);
        combined.buf = &buf[split_at_l..split_at_r];

        // Split buf into N line-aligned subslices.
        let mut boundaries = vec![0usize];
        let plen = combined.buf.len();
        let approx_step = plen / THREADS;

        for i in 1..THREADS {
            let mut idx = i * approx_step;
            while idx < plen && combined.buf[idx] != b'\n' {
                idx += 1;
            }
            if idx < plen {
                idx += 1; // include newline
            }
            boundaries.push(idx);
        }
        boundaries.push(plen);

        // Make subslices. Note: You're wasting a thread to process exactly one line
        // in leftover.
        let mut slices = Vec::<&[u8]>::new();
        slices.push(&combined.leftover);
        for w in boundaries.windows(2) {
            let (start, end) = (w[0], w[1]);
            if start < end {
                slices.push(&combined.buf[start..end]);
            }
        }

        // Compute min.
        min = scope(|scope| {
            let mut handles = Vec::new();
            for slice in slices {
                let handle = scope.spawn(move || {
                    let mut submin = 10000000.0;
                    for line in slice.split(|&b| b == b'\n') {
                        if !line.is_empty() {
                            process_line(line, &mut submin);
                        }
                    }
                    submin
                });

                handles.push(handle);
            }

            // join the handles and collect results
            handles.into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<f64>>()
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min)
        });

        // Consume buffer.
        let buf_len = buf.len();
        reader.consume(buf_len);
    }

    println!("count: {}", count);
    Ok(min)
}

// No Strings. Single threaded.
pub fn calc_min2(mut reader: BufReader<File>, count: &mut u64) -> io::Result<f64>{
    let mut min = 100000.0;

    // Used only when a line spans multiple buffers
    let mut pending = Vec::<u8>::new();

    loop {
        *count += 1;
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            // EOF: process leftover and exit
            if !pending.is_empty() {
                process_line(&pending, &mut min);
            }
            break;
        }

        if let Some(pos) = buf.iter().position(|&b| b == b'\n') {

            if pending.is_empty() {
                // Fast path: whole line inside buffer
                let line = &buf[..pos];
                process_line(line, &mut min);
            } else {
                // Slow path: line spans multiple buffers
                pending.extend_from_slice(&buf[..pos]);
                process_line(&pending, &mut min);
                pending.clear();
            }

            reader.consume(pos + 1);
        } else {
            // No newline in this chunk—part of a long line
            // Save entire buf and read more next iteration
            pending.extend_from_slice(buf);
            let len = buf.len();
            reader.consume(len);
        }
    }

    println!("count: {}", count);

    Ok(min)
}

// unchanged helper
fn process_line(line: &[u8], min: &mut f64) {
    if let Some(semi) = line.iter().position(|&b| b == b';') {
        let num_bytes = &line[semi + 1..];
        // let num_str = unsafe { std::str::from_utf8_unchecked(num_bytes) };
        let num_str = std::str::from_utf8(num_bytes).unwrap();
        let float: f64 = num_str.trim().parse().expect(num_str);
        if float <= *min {
            *min = float;
        }
    }
}

// Obvious first try. Using .lines() kills performances because it allocates a String
// for all one billion lines.
pub fn calc_min1(reader: BufReader<File>) -> io::Result<f64>{
    let mut min = 100000.0;

    // This creates a string for each line and then converts to float.
    for line_result in reader.lines(){
        let line = line_result?;
        let float: f64 = line
            .split_once(';')
            .unwrap()
            .1
            .parse()
            .unwrap();
        
        if float <= min {
            min = float;
        }
    }

    Ok(min)
}
