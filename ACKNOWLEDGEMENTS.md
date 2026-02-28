# Acknowledgments

## Engineering

- [Claude and Claude Code](https://claude.ai) ([Anthropic](https://www.anthropic.com)) — Design partner, implementation collaborator, and the reason this shipped. Claude Code executed the majority of the implementation across 50+ phases. Claude's celerity and correctness was impossible for me to match on this project. I could not have done this without it.
- [GSD (Get Shit Done)](https://github.com/gsd-build/get-shit-done) — The context engineering and planning framework that made it possible for Claude Code to execute a project of this scope coherently. Managing so many phases of implementation across a complex formal system requires more than a capable model — it requires structured context, phase planning, and assumption surfacing. GSD provided that scaffolding.

## Ideation

- [ChatGPT](https://chatgpt.com) ([OpenAI](https://openai.com)) — Spec pressure testing and convergence validation. ChatGPT's ability to rapidly iterate on spec language and provide counterexamples was invaluable for refining the spec's clarity and robustness. Its ability to break down difficult concepts and cut straight to the signal was extraordinarily helpful at validating high level thinking.
- [CUE](https://cuelang.org) — Never used directly in Tenor (unification would break the closed-world guarantees), but CUE's approach to constraints as values unlocked the mental model that made Tenor's type system possible. Marcel van Lohuizen and the CUE team did foundational work this project built on intellectually. All of the core decision protocols (invariant declaration, candidate formalism generation, counterexample pressure testing, canonical formalism selection) used are expressed in CUE.
- [DeepSeek](https://chat.deepseek.com) - Beyond spec pressure testing, DeepSeek contributed thoughtful design suggestions and was helpful as a sounding board for ideas.
