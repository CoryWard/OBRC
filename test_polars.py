import polars as pl
import time

start = time.time()

# Lazy scan
lf = pl.scan_csv(
    "measurements.txt",
    separator=";",
    has_header=False
)

# Compute min lazily.
min_val = (
    lf
    .group_by("column_1")
    .agg(
        min = pl.min("column_2"),
        max = pl.max("column_2"),
        mean = pl.mean("column_2")
    )
)

min_val.sink_csv("output_p.csv")
elapsed = time.time() - start
print("Minimum Value (subset):", min_val)
print("Runtime:", elapsed, "seconds")
