

`.LVL` File
=========

Top-level description of the level

Lines:
- Unknown integer
- Level briefing information file (see `DATA/LEVEL.TXT File`)
- Heightmap file
- Heightmap texturing file
- Default texture palette file
- Texture list file (a list of textures, see `Level Geometry`)
- `.QKE` file (UNKNOWN)
- `.PUP` file (UNKNOWN, possible same format as .PUP?)
- `.ANI` file (UNKNOWN)
- Tunnel descriptions file (see `Tunnels`)
- Sky texture
- Sky texture palette file
- Entity definitions (see `Entities`)
- `.NAV` file (UNKNOWN, nav mesh? Mission waypoints?)
- Level background music (.MOD file)
- `.FOG` file (UNKNOWN format, possibly a description of the world fog?)
- `.LTE` file (UNKNOWN format, binary visually simialr to .FOG, lighting?)
- Unknown int triple
- Unknown int
- Unknown int triple
- Unknown int
- Unknown u8
- Comment? ";New story stuff"
- Pre-level movie
- Post-level move
- Unknown filename?
- Unknown filename?
- Unknown filename?



Level Geometry
==============

- The `.RAW` file in `DATA` (linked by the main `.LVL` file) specifies the world heightmap
  - Assumtion: 256 world units per pixel, origin at middle of map.
- The `.CLR` file specifies what texture to use for each map cell
  - Each pixel corresponds to the texture for the quad to the bottom-left of the corresponding heightmap pixel
- Each texture listed in the texture list can have either a corresponding .ACT file (replace the RAW with ACT), if it doesn't the level default pallete is used.


Level Entities
==============

Consists of two sections: the type descriptions, and then entity placements


Entity types
------------
Starts with a count of entity types

Lines:
- Comma separated list of numbers and models. (Model description?)
  - Unknown small integer (u8?)
  - Unknown usually-zero integer
  - Unknown large integer (u32)
  - Unknown usually-zero integer
  - Unknown usually-zero integer
  - Unknown usually-zero integer
  - Model when active
  - Model when destroyed
- Comma separated list of numbers
  - Large-ish integer (zero, 0xFFFF, large multiple of 100, ...)
  - A u32 bitset?
  - A u32 bitset?
  - A u32 bitset?
  - unknown (usually small)
- Comma separated list of numbers, likely the drop information.
  - Unknown small
  - Unknown small
  - Percentage drop chance
  - Item ID
- Comma separated list of numbers, UNKNOWN
- ";NewHit"
- Comma separated list of numbers, UNKNOWN - usually zeroes but not always.
- "!NewAtakRet"
- Comma separated list of numbers, UNKNOWN
- Description for mission briefing
- "#New2ndweapon"
- Comma separated list of numbers, UNKNOWN
- "%SFX"
- Unknown file
- Unknown file


Entity Placements
-----------------
Starts with a count of entities

Each entitiy is a comma-seaparated list of integers
- Entity type index
- Unknown bitflag (16-bits, usually a multiple of 2^12)
- ?X Coordinate (unknown scaling)
- ?Y Coordinate
- ?Z Coordinate
- Unknown (zero?)
- Unknown (zero?)
- Unknown (often zero, sometimes u16 value)
