use criterion::{criterion_group, criterion_main, Criterion};
use std::path::Path;
use std::time::Duration;
use tdms::data_type::TdmsDataType;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("segment read, data", |b| {
        b.iter(|| {
            let file = match tdms::TDMSFile::from_path(Path::new("data/standard.tdms")) {
                Ok(f) => f,
                Err(e) => panic!("{:?}", e),
            };

            // fetch groups
            let groups = file.groups();

            for group in groups {
                // fetch an IndexSet of the group's channels
                let channels = file.channels(&group);

                for (_, channel) in channels {
                    // once you know the channel's full path (group + channel) you can ask for the full
                    // channel object. In order to fetch a channel you must call the proper channel func
                    // depending on your data type. Currently this feature is unimplemented but the method
                    // of calling this is set down for future changes
                    let full_channel = match channel.data_type {
                        // the returned full channel is an iterator over raw data
                        TdmsDataType::DoubleFloat(_) => file.channel_data_double_float(channel),
                        _ => {
                            panic!("{}", "channel for data type unimplemented")
                        }
                    };

                    let mut full_channel_iterator = match full_channel {
                        Ok(i) => i,
                        Err(e) => {
                            panic!("{:?}", e)
                        }
                    };

                    full_channel_iterator.count();
                }
            }
        })
    });
}

criterion_group! {name = benches; config = Criterion::default().measurement_time(Duration::from_secs(60)); targets = criterion_benchmark}
criterion_main!(benches);
