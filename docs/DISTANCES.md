## Distance calculation and encounter detection

This document describes the distance calculations and proximity logic used to detect
plane–drone encounters in the processing pipeline.

---

### Dataset and intermediate tables

See [SCHEMA.md](SCHEMA.md) for the definition of the underlying tables and macros.

During processing, a number of **intermediate tables** are created per site and day
(tagged to allow parallel processing):

- **`today{tag}`**  
  Plane (ADS‑B) position points within a site‑level spatial prefilter.

- **`candidates{tag}`**  
  Drone position points within the same spatial prefilter, grouped by journey.

- **`today_close{tag}`**  
  All plane/drone point pairs that are **temporally aligned and within a candidate 3D
  proximity sphere**.

- **`ids{tag}`**  
  Deterministic encounter identifiers derived from `today_close`.

The final results are written to **`airplane_prox`**.

---

### Processing pipeline (ASCII schema)

```
          ┌────────────────────┐
          │ ADS‑B (planes)     │
          └─────────┬──────────┘
                    │ spatial prefilter (≈70 NM)
                    ▼
            ┌──────────────┐
            │ today{tag}   │  (plane points, with t2)
            └───────┬──────┘
                    │ temporal join on t2 / t2+2s
                    │
            ┌───────▼──────┐
            │ today_close{tag} │  (candidate 3D sphere)
            └───────┬──────┘
                    │ final distance threshold
        ┌───────────▼───────────┐
        │        ids{tag}        │  (encounter IDs)
        └───────────┬───────────┘
                    │
                    ▼
          ┌────────────────────┐
          │   airplane_prox    │  (final encounters)
          └────────────────────┘

          ┌────────────────────┐
          │ Drone positions    │
          └─────────┬──────────┘
                    │ spatial prefilter (≈70 NM)
                    ▼
            ┌──────────────┐
            │ candidates{tag} │  (drone points, journeys, with t2)
            └──────────────┘
```

The pipeline is strictly left‑to‑right: expensive joins and distance calculations are
performed once and reused deterministically downstream.

---

### Spatial and temporal prefiltering

For a given site and day:

1. **Plane points**
    - All ADS‑B points are selected within a **70 NM bounding ellipse** around the site.
    - A 2‑second time bucket (`t2`) is computed:
      ```
      t2 = toStartOfInterval(time, 2s)
      ```

2. **Drone points**
    - All drone points for the same day are selected within the same **70 NM bounding ellipse**.
    - The same 2‑second bucket (`t2`) is computed from the drone timestamp.
    - Per‑journey grouping is preserved.

These steps define the **maximum search space** and are purely performance‑driven.

---

### Temporal alignment

Plane and drone points are considered temporally compatible if their 2‑second buckets
match exactly or with a +2 second tolerance:

```
t.t2 = c.t2 OR t.t2 = addSeconds(c.t2, 2)
```

This implements a **±2 second temporal alignment** without row duplication.

---

### Distance computation (core semantics)

Distance calculations follow a **true 3D spherical model**:

- The **only correctness criterion** for proximity is the **3D slant distance**.
- The distance is computed as a squared value:

```
dist_3d_sq(drone_lon, drone_lat, drone_alt,
           plane_lon, plane_lat, plane_alt)
```

An encounter candidate must satisfy:

```
dist_3d_sq <= R²
```

> **Important:**
> - 2D distance and altitude difference are **not correctness gates**.
> - They may be used only as **prefilters or diagnostics**.

---

### Two‑threshold model

The pipeline intentionally uses **two distance thresholds**:

1. **Candidate threshold**
   ```
   separation = threshold × factor
   ```
   Used to populate `today_close{tag}` conservatively.

2. **Final encounter threshold**
   ```
   threshold
   ```
   Applied when generating encounter IDs and inserting into `airplane_prox`.

---

### Encounter generation

From `today_close{tag}`:

1. All pairs with:
   ```
   dist_drone_plane < threshold
   ```
   are considered true encounters.

2. A deterministic encounter ID is generated per:
   ```
   (site, day, journey, plane)
   ```

3. Final encounter records include:
    - timestamp
    - 3D slant distance (meters)
    - drone identifiers and position
    - plane identifiers (callsign, Mode‑S) and position

---

### Design notes

- `today_close{tag}` is **materialized as an intermediate table**, not a CTE, so that
  the expensive join and distance computation is executed exactly once and reused.
- Ordering is not relied upon for correctness; downstream stages re‑group explicitly.

This design prioritises correctness (true 3D distance), reproducibility, and predictable
performance at scale.
