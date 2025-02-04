# `compute-height`

Geoid height and geometric altitude are concepts from geodesy, and they deal with how we measure height above the
Earth's surface. Understanding the distinction between them is essential to various applications, including GPS systems
and geoid models like EGM2008.

### 1. Geoid Height

The **geoid** is an equipotential surface of the Earth's gravity field, approximating the mean sea level across the
globe. This surface is not flat because the Earth's mass distribution is irregular, causing gravity to vary spatially.
The **geoid height** (or geoid undulation) is the distance between the geoid and the reference ellipsoid (a
mathematically defined, smooth, oblate spheroid used to approximate the Earth's shape).

- **Positive geoid height**: The geoid is above the ellipsoid.
- **Negative geoid height**: The geoid is below the ellipsoid.

Geoid height depends on a location's latitude and longitude and is measured in models such as **EGM2008** (Earth
Gravitational Model 2008).

### 2. Geometric Altitude

The **geometric altitude** is the height above the reference ellipsoid, also called the ellipsoidal height. Geometric
altitude can be directly measured by GPS receivers because they use an ellipsoid as the reference surface.

### Key Difference

- **Geoid height** is the deviation of the geoid from the ellipsoid (measured perpendicular to the ellipsoid). It
  represents the height adjustments due to Earth's irregular gravity field.
- **Geometric altitude** is the direct vertical distance above the ellipsoid.

### Relationship Between Them:

To derive the **orthometric height** (commonly used height above sea level) from **geometric altitude**, you need to
account for the geoid height:

```plain text
Orthometric Height (H) = Geometric Altitude (h) - Geoid Height (N)
```

In this equation:

- `H`: Height above mean sea level (orthometric height).
- `h`: Height above the ellipsoid (geometric altitude).
- `N`: Geoid height.

### Practical Implications

- GPS provides **geometric altitude**, but not directly the orthometric height. To get orthometric height, you need
  geoid height (e.g., using geoid models like EGM2008).
- Geoid models help refine altitude measurements by factoring in Earth's gravitational irregularities for precise
  applications.

The code snippet in context indicates a tool for calculating **geoid heights** using the **EGM2008 geoid model**. The
`geoid_height()` function likely computes the deviation of the geoid from the ellipsoid for a given latitude and
longitude. This result can then aid in determining orthometric height.
