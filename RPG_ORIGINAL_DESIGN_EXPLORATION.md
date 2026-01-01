# File-System RPG Design Summary

## Core Concept
A text-based RPG built into a terminal interface where the file system itself *is* the game. Players navigate using only three basic commands:
- `ls` - see available options
- `cd` - take actions/navigate
- `cat` - examine items/status

## File System Conventions

### File Types
- `.nme` - Enemy files (contains stats, can be examined)
- `.wpn` - Weapon items
- `.arm` - Armor items
- `.itm` - Generic items/loot
- `.fam` - Familiar/companion (tamed creatures)
- `.pet` - Pet files (alternative to .fam, used for slimes)
- `.txt` - Status files, lore, descriptions

### Actions as Directories
Combat and interaction options appear as directories:
- `attack_[enemy]/` - Attack a specific enemy
- `tame_[enemy]/` - Attempt to tame a creature
- `defend/` - Defensive action
- `rest/` - Heal/recover

## Core Mechanics

### Combat System
- Turn-based: player attacks, enemy counterattacks
- HP and power stats determine damage
- Armor provides damage reduction
- Items drop as loot after victory
- XP gained for defeating enemies

### Taming System
**Key principle: ALL enemies (.nme files) are theoretically tameable**

- Each enemy has a base taming percentage
- Probability affected by: player level, items, enemy HP, creature type
- Slimes: Easy to tame (~80%), weak initially but scale into late-game power
- Goblins: Technically tameable but very low % and poor scaling (meme option)
- Bosses: Extremely low % but not impossible

**No untameable enemies** - just varying degrees of difficulty

### Familiar/Companion System
Organization: `inventory/familiars/[creature].fam/`

Each familiar directory contains:
- `status.txt` - Current stats
- `bond_level.txt` - Relationship progression
- `abilities/` - Available skills
- `feed/`, `train/` - Interaction options

**Bond Mechanics:**
- Increases with combat usage
- Damaged if familiar takes heavy damage while player is healthy
- Shared struggle (both low HP) doesn't hurt bond
- Attitude progression: tame → friendly → loyal → devoted → bonded
- Higher bonds unlock stat boosts and abilities

### Slime Evolution Design Goal
- Tamed early when weak
- Initially appears suboptimal
- Scales aggressively into endgame
- Can become strongest companion with investment
- Inspired by isekai anime tropes

## Technical Implementation

### Stack
- **Rust** compiled to **Wasm** for game logic
- Minimal **JavaScript** wrapper for terminal interface
- **wasm-bindgen** + **web-sys** for JS interop
- **getrandom** crate (with wasm features) for RNG
- Hosted on **GitHub Pages** (static, client-side only)

### Terminal Interface
- Custom-built terminal emulator
- Autocomplete planned (challenging on mobile)
- Minimal aesthetic
- Must work on both desktop and mobile

## Design Challenges

1. **Procedural Generation** - Need system to generate encounters/locations on demand
2. **Mobile Autocomplete** - Tab completion doesn't translate well to touch
3. **Content Scale** - Every enemy needs stats, loot tables, etc. (but this is true of all RPGs)
4. **Flavor Text** - Minimal interface requires good descriptive text for engagement
5. **Feature Creep** - Temptation to add infinite systems (crafting, skill trees, etc.)

## File Structure Example
```
/
├── player_status.txt
├── [enemy].nme
├── inventory/
│   ├── [weapon].wpn
│   ├── [armor].arm
│   ├── familiars/
│   │   ├── slime.fam/
│   │   │   ├── status.txt
│   │   │   ├── bond_level.txt
│   │   │   └── abilities/
│   │   └── dragon.fam/
│   └── map/
│       ├── forest/
│       └── road_1/
├── attack_[enemy]/
├── tame_[enemy]/
├── defend/
└── rest/
```

## Scope Philosophy
- Keep it minimal - this is a portfolio project
- Single-player only (no multiplayer complexity)
- No human enemies (simplifies taming ethics)
- Everything accessible through consistent file metaphor
- Designed to sit on personal website as an easter egg

## Future Possibilities (If Feature Creep Wins)
- Crafting: `cd /forge/combine/[items]/`
- Skill trees: `.skill` files
- Achievements: `.badge` files in `/trophies/`
- Multiple save slots: export/import functionality
- Localization (brother is professional game translator JP→EN)

## Development Status
- ✅ Terminal emulator built
- ✅ Wasm compilation pipeline ready
- ✅ Core mechanics designed
- ⏳ Actual game logic: not started
- ❓ Will it ever be finished: TBD