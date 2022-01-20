# Getting Started
Easy enough to run
```
cargo run --release
```

## Change generation speed
Find the system set that looks like this
```
.add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(2.00))
                .with_system(random),
        )
```

and change the value from `2.00` to whatever you want in seconds.

## Change the map size
Adjust the layer settings.
```
let layer_settings = LayerSettings::new(
        MapSize(2, 2),
        ChunkSize(64, 64),
        TileSize(16.0, 16.0),
        TextureSize(96.0, 16.0),
    );
```

The map above is `(2*64)^2`.  

![alt text](./screenshots/1.png?raw=true)
![alt text](./screenshots/2.png?raw=true)
![alt text](./screenshots/3.png?raw=true)

