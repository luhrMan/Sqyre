#!/usr/bin/env python3
"""Compare two sqyre-bench-compare JSON reports (Go↔Rust or baseline↔current)."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def load(path: Path) -> dict[str, Any]:
    with path.open() as f:
        return json.load(f)


def ns(v: Any) -> float:
    try:
        return float(v)
    except (TypeError, ValueError):
        return 0.0


def fmt_ns(v: float) -> str:
    if v >= 1e9:
        return f"{v / 1e9:.2f}s"
    if v >= 1e6:
        return f"{v / 1e6:.2f}ms"
    if v >= 1e3:
        return f"{v / 1e3:.1f}µs"
    return f"{int(v)}ns"


def pct(new: float, old: float) -> str:
    if old <= 0:
        return "n/a"
    return f"{(new - old) / old * 100:+.1f}%"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("left", type=Path, help="baseline / Go / older JSON")
    ap.add_argument("right", type=Path, help="current / Rust / newer JSON")
    ap.add_argument(
        "--label-left",
        default=None,
        help="column label for left (default: impl_name or file stem)",
    )
    ap.add_argument(
        "--label-right",
        default=None,
        help="column label for right",
    )
    ap.add_argument(
        "--wall-regress-pct",
        type=float,
        default=10.0,
        help="flag wall/iter regression above this %% (default 10)",
    )
    ap.add_argument(
        "--rss-regress-pct",
        type=float,
        default=15.0,
        help="flag peak_rss_kb regression above this %% (default 15)",
    )
    ap.add_argument(
        "--fail-on-regress",
        action="store_true",
        help="exit 1 when wall/rss regressions exceed thresholds (for baseline diffs)",
    )
    args = ap.parse_args()

    left = load(args.left)
    right = load(args.right)
    lname = args.label_left or left.get("impl_name") or args.left.stem
    rname = args.label_right or right.get("impl_name") or args.right.stem

    lsec = {s["name"]: s for s in left.get("sections", [])}
    rsec = {s["name"]: s for s in right.get("sections", [])}
    names = sorted(set(lsec) | set(rsec))

    print(
        f"compare  {lname} ({left.get('git_describe', '?')})  vs  "
        f"{rname} ({right.get('git_describe', '?')})"
    )
    print(
        f"{'section':<24} {lname + ' wall':>14} {rname + ' wall':>14} "
        f"{'Δwall':>9} {lname + ' rss':>10} {rname + ' rss':>10} {'Δrss':>9} flags"
    )

    regressions = 0
    for name in names:
        a = lsec.get(name)
        b = rsec.get(name)
        if not a or not b:
            missing = lname if not a else rname
            print(f"{name:<24}  (missing on {missing})")
            continue
        if a.get("status") != "ok" or b.get("status") != "ok":
            print(
                f"{name:<24}  {a.get('status')}/{b.get('status')}  "
                f"{a.get('skip_reason') or ''} {b.get('skip_reason') or ''}".rstrip()
            )
            continue

        aw = ns(a.get("wall_ns_per_iter"))
        bw = ns(b.get("wall_ns_per_iter"))
        ar = ns(a.get("peak_rss_kb"))
        br = ns(b.get("peak_rss_kb"))
        flags = []
        # For baseline→current, right slower (higher wall) is a regression.
        if aw > 0 and (bw - aw) / aw * 100 >= args.wall_regress_pct:
            flags.append("WALL↑")
            regressions += 1
        if ar > 0 and (br - ar) / ar * 100 >= args.rss_regress_pct:
            flags.append("RSS↑")
            regressions += 1
        if aw > 0 and (bw - aw) / aw * 100 <= -args.wall_regress_pct:
            flags.append("wall↓")
        if ar > 0 and (br - ar) / ar * 100 <= -args.rss_regress_pct:
            flags.append("rss↓")

        print(
            f"{name:<24} {fmt_ns(aw):>14} {fmt_ns(bw):>14} {pct(bw, aw):>9} "
            f"{int(ar):>10} {int(br):>10} {pct(br, ar):>9} {' '.join(flags)}"
        )

        # I/O line when either side saw disk traffic
        aio = ns(a.get("io_read_bytes")) + ns(a.get("io_write_bytes"))
        bio = ns(b.get("io_read_bytes")) + ns(b.get("io_write_bytes"))
        if aio or bio:
            print(
                f"{'':<24}   io_rw {int(aio)} → {int(bio)}  ({pct(bio, aio) if aio else 'n/a'})"
            )

    print()
    if regressions:
        print(f"{regressions} regression flag(s) above thresholds "
              f"(wall {args.wall_regress_pct}%, rss {args.rss_regress_pct}%).")
        if args.fail_on_regress:
            return 1
        return 0
    print("No regressions above thresholds.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
