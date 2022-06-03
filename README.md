# SEP-Data (WIP)

A library for receiving Smart Eye Pro (SEP) output data via TCP/UDP.

## Generating output_data for new SEP version

Use generate_output_data.py to generate a new output_data.rs:

```
> py .\scripts\generate_output_data.py "C:\Program Files\Smart Eye\Smart Eye Pro X.Y\API\include\data_output.json" .\src\se_types\output_data.rs
> cargo fmt -- .\src\se_types\output_data.rs
```
