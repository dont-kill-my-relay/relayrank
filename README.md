# Relay Rank

This repository contains the code to compute the relay-adversary and network-adversary metrics for relays in the Tor network.
The details and reasoning of those metrics can be found in the associated scientific paper:
J. Dejaeghere, L. Goffaux, P. Luycx, H. Elkoulak, and F. Rochet. 2026. Sociotechnical Aspects of Tor Relay Rejection.
In *25th Workshop on Privacy in the Electronic Society (WPES '26)*. [doi:10.1145/3847192.3847370](https://doi.org/10.1145/3847192.3847370).

To cite our results or method, please use the following citation entry.

<details>
<summary>Show bibtex</summary>
<pre>
@inproceedings{dejaeghere_sociotechnical_2026,
    author = {Dejaeghere, Jules and Goffaux, Lionel and Luycx, Pierre and Elkoulak, Hosam and Rochet, Florentin},
    title = {{Sociotechnical Aspects of Tor Relay Rejection}},
    year = {2026},
    month = {nov},
    publisher = {Association for Computing Machinery},
    address = {New York, NY, USA},
    doi = {10.1145/3847192.3847370},
    booktitle = {Proceedings of the 25th Workshop on Privacy in the Electronic Society (WPES '26)},
    keywords = {Tor; Software release life cycle; Anonymity; Tor relay; Internet exchange point (IXP); Autonomous system (AS)},
    location = {The Hague, Netherlands},
    series = {WPES '26},
}
</pre>
</details>

## Usage

Each metric uses a set of arguments that are the same for all metrics:
  - The output file. This will be the name of the `csv` file where the final ranking will be written.
  - The date and time for which you want to compute a metric. This program always works in the UTC time zone. The format is a relaxed form of RFC3339. See the [chrono documentation](https://docs.rs/chrono/latest/chrono/struct.DateTime.html#impl-FromStr-for-DateTime%3CUtc%3E) for more details.
  - The cache folder. This is optional. By default, `./cache/` is used.

> [!IMPORTANT]
> The cache folder must contain the consensuses and relay descriptors for the date and time requested.

> [!TIP]
> When the metrics are computed early in a month, the relay descriptor associated with a consensus can be in the previous month. Make sure to download it as well.

See the [relay-rejection](https://github.com/dont-kill-my-relay/relay-rejection) repository for how to build the cache folder.

<details>
<summary>Example of cache directory structure:</summary>
<pre>
- `cache/`
  - `consensuses/`
    - `2021-11-01-00-00-00-consensus`
    - `2021-11-01-01-00-00-consensus`
    - `2021-11-01-02-00-00-consensus`
    - `2021-11-01-03-00-00-consensus`
    - ...
  - `relay_descriptors/`
    - `server-descriptors-2021-11/`
      - `0102a...`
      - `af0b6...`
      - `be562...`
      - ...
    - `server-descriptors-2021-12/`
    - ...
</pre>
</details>

You can then use the appropriate subcommand depending on your metric.

```bash
./relay-rank -o=metric.csv "2022-03-19T12:00:00Z" <sub-command>
```

### Relay-adversary metric

The `relay-metric` subcommand computes the relay-adversary metric.
It takes only the exclusion list as an optional argument.

```bash
./relay-rank -o=network-metric.csv "2022-03-19T12:00:00Z" \
    relay-metric [exclusion-list]
```

If an exclusion list is provided, the output contains only the ranking of the excluded relays.

### Network-adversary metric

To compute the network-adversary metric, the `network-metric` subcommand is available.
This subcommand takes three arguments:
  - The mapping between the inferred AS-paths and the relay IDs of the corresponding guard or exit.
  - The result of the AS-path inference.
  - The exclusion list (optional).

```bash
./relay-rank -o=network-metric.csv "2022-03-19T12:00:00Z" \
    network-metric <mapping-file> <inference-result> [exclusion-list]
```

The mapping is obtained with the `utils/build_inference_file_net_metric.py` script in the
[relay-rejection](https://github.com/dont-kill-my-relay/relay-rejection/blob/main/utils/build_inference_file_net_metric.py) repository.
That script also generates the input for the AS-path inference that can then be used to
produce the second argument.
See the [as-path-inference](https://github.com/dont-kill-my-relay/as-path-inference)
repository to infer the AS-path.
If an exclusion list is provided, the output contains only the ranking of the excluded relays.
