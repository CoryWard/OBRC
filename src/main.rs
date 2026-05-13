// One Billion Row Challenge.
//
// Standard library only.

use core::f64;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::time::Instant;
use std::thread::{scope};
use std::collections::{HashMap};

const THREADS: usize = 14;

struct TempBuff<'a>{
    leftover: Vec<u8>,
    buf: &'a [u8]
}

struct Stats{
    min: f64,
    max: f64,
    sum: f64,
    count: f64,
}

impl Stats{
    fn new() -> Stats{
        Stats{
            min: f64::INFINITY,
            max: -f64::INFINITY,
            sum: 0.0,
            count: 0.0,
        }
    }

    fn update(&mut self, other: Stats) {
        self.min = f64::min(self.min, other.min);
        self.max = f64::max(self.max, other.max);
        self.sum = self.sum + other.sum;
        self.count += other.count;
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.min, self.max, self.sum/self.count, self.count)
    }
}

fn main() -> io::Result<()> {
    let start = Instant::now();
    // Scan file and calculate minimum temperature.
    let file = File::open("measurements.txt")?;
    let reader = io::BufReader::with_capacity(1000 * 1000 * 1024, file);

    calc_stats(reader)?;

    let duration = start.elapsed();   // <-- stop timer
    println!("Runtime: {:?}", duration);
    Ok(())
}

pub fn calc_stats(
    mut reader: BufReader<File>,
) -> io::Result<()>{
    let mut leftover = Vec::<u8>::new();

    let mut all_stats = HashMap::new();

    loop {
        let buf = reader.fill_buf()?;

        if buf.is_empty() & leftover.is_empty(){
            break;
        }

        let mut combined = TempBuff{leftover: leftover.clone(), buf: buf};

        let slices = get_chuncks(buf, &mut leftover, &mut combined);
    
        // Compute stats.
        scope(|scope| {
            let mut handles = Vec::new();
            for slice in slices {
                let handle = scope.spawn(move || {
                    let mut stats = HashMap::<String, Stats>::new();
                    for line in slice.split(|&b| b == b'\n') {
                        if !line.is_empty() {
                            process_line(line, &mut stats);
                        }
                    }
                    stats
                });

                handles.push(handle);
            }

            for handle in handles {
                let stat_map = handle.join().unwrap();
                for (key, value) in stat_map.into_iter() {
                    if !all_stats.contains_key(&key) {
                        all_stats.insert(key, value);
                    }
                    else {
                        all_stats.get_mut(&key).unwrap().update(value);
                        
                    }
                }

            }
        });

        // Consume buffer.
        let buf_len = buf.len();
        reader.consume(buf_len);
    };

    write_map(&all_stats);
    Ok(())
}

fn get_chuncks<'a, 'b, 'c>(
    buf: &'a[u8],
    leftover: &mut Vec<u8>,
    combined: &'b mut TempBuff<'b>,
) -> Vec<&'c[u8]>
    where 'a:'b, 'b:'c
{
    let split_at_l = combined
        .buf
        .iter()
        .position(|&b| b == b'\n')
        .map(|idx| idx + 1)
        .unwrap_or(0);

    // Find last newline in buf.
    let split_at_r = combined
        .buf
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|idx| idx + 1)
        .unwrap_or(0);           

    // Get partial line from buf.
    leftover.clear();
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
    
    slices
}

fn process_line(
    line: &[u8],
    stats: &mut HashMap<String, Stats>,
) {
    if let Some(semi) = line.iter().position(|&b| b == b';') {
        let city_bytes = &line[..semi];
        let temp_bytes = &line[semi + 1..];
        // let num_str = unsafe { std::str::from_utf8_unchecked(num_bytes) };
        let city_str = std::str::from_utf8(city_bytes).unwrap();
        let num_str = std::str::from_utf8(temp_bytes).unwrap();
        let float: f64 = num_str.trim().parse().expect(num_str);

        if !stats.contains_key(city_str) {
            stats.insert(city_str.to_string(), Stats::new());
        }

        let city_stat = stats.get_mut(city_str).unwrap();

        if float < city_stat.min {
            city_stat.min = float;
        }

        if city_stat.max < float {
            city_stat.max = float;
        }

        city_stat.sum = city_stat.sum + float;
        city_stat.count += 1.0;
    }
}

fn write_map(map: &HashMap<String, Stats>) -> io::Result<()> {
    let mut file = File::create("output.txt")?;

    for (k, v) in map {
        writeln!(file, "{}\t{}", k, v)?;
    }

    Ok(())
}