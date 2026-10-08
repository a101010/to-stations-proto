# Earth cube-map faces

## Source

The six faces are derived from NASA's **Blue Marble (2002)** — land surface and ocean color — which is in the **public domain** (NASA material is generally not protected by copyright).

- Source image: *Whole world – land and oceans* (8192×4096 equirectangular), Wikimedia Commons:
  <https://commons.wikimedia.org/wiki/File:Whole_world_-_land_and_oceans.jpg>
- Original data: NASA Goddard Space Flight Center; image by Reto Stöckli (land surface, shallow water, clouds); enhancements by Robert Simmon (ocean color, compositing, 3D globes, animation). See NASA Visible Earth, "The Blue Marble: Land Surface, Ocean Color, Sea Ice and Clouds".

## Conversion

Produced with `rust/tool/equirect-to-cubemap`:

```
equirect-to-cubemap <blue-marble-equirect.jpg> assets/earth --size 1024 --format jpg --target-bytes 80000
```

`--target-bytes 80000` selects, per face, the highest JPEG quality whose encoded size is at or below 80 KB, so every committed face stays within the 50–100 KB budget.

| Face | File | Size (bytes) | JPEG quality |
|---|---|---|---|
| +X | `px.jpg` | 79143 | 39 |
| -X | `nx.jpg` | 79993 | 65 |
| +Y | `py.jpg` | 79646 | 33 |
| -Y | `ny.jpg` | 79478 | 82 |
| +Z | `pz.jpg` | 79819 | 46 |
| -Z | `nz.jpg` | 77882 | 89 |

## Face order and orientation

- Faces are named and ordered `px, nx, py, ny, pz, nz` (standard OpenGL cube-map layout).
- For point `(s, t)` in `[-1, 1]` on a face, the direction is:

  | Face | Direction |
  |---|---|
  | `+X` (`px`) | `( 1, -t, -s)` |
  | `-X` (`nx`) | `(-1, -t,  s)` |
  | `+Y` (`py`) | `( s,  1,  t)` |
  | `-Y` (`ny`) | `( s, -1, -t)` |
  | `+Z` (`pz`) | `( s, -t,  1)` |
  | `-Z` (`nz`) | `(-s, -t, -1)` |

  where `s` increases to the right and `t` increases downward in each face image.
- Equirectangular mapping: `+Y` is north; `u = 0.5` is longitude `0` at the `+Z` face center; `v = 0` is the north pole. Longitude wraps in `u`; latitude clamps in `v`.
- Absolute orientation is arbitrary: `earth-rendering` applies the attitude quaternion. It must sample the faces using the convention above.
