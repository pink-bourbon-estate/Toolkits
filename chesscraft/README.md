# ChessCraft

A system-first chess opening trainer for two repertoires: **the Modern Defence as Black** and **the London System as White**. The idea is to learn one set of moves that holds up against most of what the opponent tries, rather than memorising a different reply for every line from day one.

Written in Rust with egui, the same toolkit as PianoCraft. Local-first: no accounts, no cloud, and your progress stays on your computer.

## Install (Windows)
1. Download `chesscraft.exe` from the newest **ChessCraft v0.1.x** release on the [Releases page](https://github.com/pink-bourbon-estate/toolkits/releases).
2. Run it. If SmartScreen warns about an unknown app, click **More info → Run anyway** (it isn't code-signed).
3. Click **Install ChessCraft** on the bottom bar. It copies itself to `%LOCALAPPDATA%\Programs\ChessCraft` (no admin rights), adds it to the **Start menu** and to **Settings → Apps** (uninstall from there), then you can delete the downloaded file.

## Updates
Every change pushed to `chesscraft/` is built by GitHub Actions and published as release `chesscraft-v0.1.N`, with SHA-256 checksums. Click **Check for updates** in the top bar; ChessCraft downloads the new build, verifies the checksum, replaces itself and restarts. The only network request is an anonymous download from github.com — no account, no telemetry, nothing about you is sent.

## How the repertoire was built
Every line was generated and checked by a real **Stockfish 19** engine, not written from memory:
- Our own moves come from a declared plan, but only where the engine agrees that move is within a small centipawn tolerance of its own best move. Where the plan's move is meaningfully worse, ChessCraft plays the engine's actual best move instead and marks the node as a **deviation**, with the centipawn cost it would have taken to stick with the plan.
- The opponent's replies are the engine's own top candidates (multipv), so the tree only branches where a real engine thinks there's a real choice.
- A deeper, independent audit pass re-checks every line after generation to catch anything the generation depth missed.

ChessCraft shows this honestly in the app: each repertoire reports what fraction of your own positions are the plain "system" move versus an engine-forced deviation, rather than claiming 100% coverage.

## Modes
- **Explore** — click through the whole saved tree freely, branch by branch, with the engine's note on every node.
- **Learn** — plays move by move with explanations, so you build the plan's ideas rather than just memorising notation.
- **Drill** — spaced repetition: ChessCraft shows you a position from your repertoire where it's your move, you find it on the board, and it schedules the next review based on whether you got it right.

## Your data
Progress and spaced-repetition scheduling live in your own app-data folder (`progress.json`) and are never committed to this repository — see the repo's top-level `.gitignore` and the Privacy and security section in the main README.
