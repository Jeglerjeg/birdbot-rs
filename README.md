# TODO:

### Map formatting:
- Diff changes during update

### Tracking:
- Scoresaber?

### Unimplemented:
- Replay rendering
- Custom PP calculation

### Maybe implement?:
- Minigames
- Image editing
- Pokedex
- Wordsearch
- Moderation
- Twitch
- Countdowns
- Urbandictionary

### Won't implement:
- Brainfuck
- Dailies
- Pranks
- Pastas

## Configuration

| ENV variable      | Accepted value                                                 |
|-------------------|----------------------------------------------------------------|
| DATABASE_URL      | Postgres connection URL                                        |
| DISCORD_TOKEN     | Discord token                                                  |
| PREFIX            | Default bot prefix                                             |
| OSU_CLIENT_ID     | osu! apiv2 client ID                                           |
| OSU_CLIENT_SECRET | osu! apiv2 client secret                                       |
| SCORES_WS_URL     | URL to scores-ws instance, defaults to ws://127.0.0.1:7727     |
| UPDATE INTERVAL   | How often the osu tracking loop is run. Defaults to 30 seconds |
| NOT_PLAYING_SKIP  | Skip updating non-playing users for N runs. Defaults to 10     |
| MAX_SONGS_QUEUED  | Max amount of songs queued per person. Defaults to 6           |