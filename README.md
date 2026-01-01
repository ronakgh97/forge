# forge

**forge** is a low-level, no BS, opinionated, model-agnostic agent runtime designed for building
tool-using AI systems in a clear, explicit, and controllable way.

> Think cool design, not slop product.

## Why I made this?

Well, actually I made this on the fly, for projects that I was working on, local updating was a pain, so it decided to
make a separate
repo for it.
I didn't find any existing libraries/crate that matched what control, API clarity and features I need and how
an agent runtime should work, ***they are too high level/so much abstracted/ PRODUCT ORIENTATE SLOP, too strict in their
design,*** so I decided to make my own.

Modern AI tooling often hides too much behind abstractions.
That’s convenient at first — and painful later.

forge is built with a more traditional philosophy:

- Explicit state over hidden chains
- Simple data structures over heavy frameworks
- Composition over inheritance
- You control the loop, stream, the model, and the tools

If you like knowing **exactly** what your agent is doing at every step,
this library is for you.

## Little Overview

forge is centered around a few stable ideas:

- **Agent Runtime**  
  The execution loop that drives reasoning and tool use.

- **Tools**  
  Explicit, callable capabilities with well-defined inputs and outputs.

- **Sessions / State**  
  Clear separation between agent logic and conversational or task state.

- **Streaming-first**  
  Designed to handle incremental model outputs cleanly.

- **Model-agnostic**  
  Works with local models, hosted APIs, or anything that can produce tokens.

## Don'ts

forge intentionally does **not** try to be:

- A chatbot UI
- A SaaS wrapper
- A prompt engineering playground
- A single-model solution

Those can be built *on top* of it.

## Usage

This library is **not published on crates.io**, I hate versioning and release cycles.

Add it directly from GitHub:

```toml
[dependencies]
forge = { git = "https://github.com/ronakgh97/forge" }
```

## Example

Will update this later...
Check these repos for now:

- [R-Agent](https://github.com/ronakgh97/r_agent): An experimental project showcasing the capabilities of `forge` in
  building AI agents.
- [Pokebrains](https://github.com/ronakgh97/pokebrains): A project utilizing `forge` for creating intelligent Pokemon
  Showdown move suggestion during online Team Battle, and Showdown format TEAM generation agent.

## License

This project is licensed under the MIT License.

## Contribution?

No thanks...I am too depressed/insecure to maintain open source projects, and collaborate with strangers.
If you insist?!, then sure, I guess whatever.

## Final Thoughts

**Lastly....AI SLOP is too overhyped/immauture and will not replace the coding as much as people think.**


