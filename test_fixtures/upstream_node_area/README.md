# Upstream node area fixture

This 8 × 8 UTM grid has twelve watershed cells of 100 × 100 m. The sketch gives zero-based (row, column) coordinates via the axes. Arrows show D8 directions. Uppercase letters are stream cells.

    row   col 1  2  3  4  5  6
      1       ↓           ↓
      2       A           B  ←
      3          C     D
      4             J
      5          →  E
      6             O  ←

A flows southeast through C to J. B flows southwest through D to J. J flows south through E to the outlet O. The two headwater nodes receive one and two cells respectively. J receives seven cells (two upstream channel cells on each branch plus three source cells). The two lateral cells below J enter E and O. Including O itself the basin has twelve cells. The expected upstream area at J is 70,000 m². Its local lateral area is 20,000 m².
