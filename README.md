# >_ cosmonaut code

[![Rust Check](https://github.com/cosmonaut-nz/cosmonaut-code/actions/workflows/rust-check.yml/badge.svg)](https://github.com/cosmonaut-nz/cosmonaut-code/actions/workflows/rust-check.yml)

## purpose

it's a code explorer, explainer and assessment tool.

to be honest, we built this tool because we needed it for the work we do. sure there are some great tools out there, but none of them quite hit the mark for our needs.

generative ai will maximise outputs and 10x developer output. what about the quality? this tool aims to address this - i.e., use the source of the problem to provide a solution.

### goals

1. provide a viable tool for local codebase analysis.
2. helps new developers to quickly get up to speed on a large or legacy codebase.
3. provide a tool that will help developers and code maintainers manage their code and start a conversation on quality.
4. allow code owners to check the overall health of the code in a simple way.
5. output an actionable report with auto-PRs that will improve the code base over time.

### non-goals

1. ~~provide a tool that will automagically improve a codebase.~~
2. provide a tool that does not require further conversation, review or analysis of the code.
3. provide a tool that does not require thinking or discussion.

### use cases

1. new developers on a project.
2. new maintainers of a codebase or take over of code base / project.
3. entry-point for a formal external audit of code base.
4. entry-point for a formal external security audit of code base.
5. entry-point for due diligence of technology assets.
6. code owner reporting on technical-debt and general health of asset.

## current state

currently, most things are working and a solid report is produced in either `json` or `html`.

the most stable and tested provider is openai. The best results, by far, are with the `gpt-4` service, which uses the latest `preview` model. the openai `gpt-3.5` works, but tends to over state issues and the quality of resolution offered to isses is not as good. it does run faster, however and is cheaper to run.

google are late to the party, but have come in the door with a half-drunken bottle. the public instance of `gemini-pro` is both faster, cheaper and produces better results that openai's `gpt-3.5`. it is slightly behind the `gpt-4` `preview` model, but not far. do your own testing; we've found the late 2023 comparisons online to be highly misleading. the google provider is not as tested as openai, so you will see more errors in the log output. it should recover from these errors, but it is less robust.

## disclaimer

this is really early days. running over a really big repo with the latest model will be super slow and possibly fail. we've tested it up to ~1500 code files, what with timeout retries etc., takes a couple of hours, cost about 5 usd. your mileage may vary. we think the value will come when it can be run over multiple models and compared and filtered.

as with all similar tools, it does produce false flags. it overplays or (rarely) downplays security issues. in some cases it may flag so many issues that the response is truncated, creating an error. we are working on this.

there is significant variation between models and even review runs on the same repository with the same model, particularly with older models. some models are silent on obvious issues and transfixed on trivial issues.

there are issues with the language file type matching via the github linguist regex. we will likely move to something more robust, or fix the crate that causes the mismatching.

we recommend that you run it multiple times at first to gain a base line; fix the big issues and then let it run periodically.

right now it's deliberately a barebones offering. it works well, and we have gotten value from it, but there is a lot more to do. it's been fun to do.

the google public api provider works, but is less robust than openai.

there is a local instance wired up. it does work, but it highly fragile and unlikely to complete. it currently uses lm studio.

## usage

download pre-release

[MacOS Apple Silicon](https://github.com/cosmonaut-nz/cosmonaut-code/releases/download/v0.2.0/cosmonaut_code_0.2.0_macos-aarch64)

### configuration

configure: add a `settings.json`, maybe in the `settings` folder, with the following:

```json

{
    "sensitive": {
        "api_key": "[YOUR_API_KEY]"
    },
    "repository_path": "[FULL_PATH_TO_REPO]",
    "report_output_path": "[FULL_PATH_TO_OUTPUT]",
    "chosen_provider": "[CHOICE OF PROVIDER]",
    "chosen_service": "[CHOICE OF SERVICE]",
    "output_type": "html"
}

```

`chosen_provider` is in:

1. `openai` (default)
2. `google` (note API key only, ADC does not work as this is the public version)

`chosen_service` is in:

1. `gpt-4` (default)
2. `gpt-3.5`
3. `gemini-pro` (for google provider)

`output_type` is in:

1. `html`
2. `json` - (default)

run:

```bash

export SENSITIVE_SETTINGS_PATH=[PATH_TO_YOUR_SETTINGS.JSON]

```

download release above

```bash

mv cosmonaut_code_0.2.0_macos-aarch64 cosmonaut_code

```

```bash

./cosmonaut_code

```

## via rust locally

### tldr

install rust; clone the repo; cd repo; add config (see above); `cargo run`.

```bash

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

```

```bash

git clone https://github.com/cosmonaut-nz/cosmonaut-code.git


```

```bash

cd cosmonaut-code

```

Add settings (as per above)

```bash

cargo run

```

## contributing

yes please!!

see [contributing](CONTRIBUTING.md) for the rules, they are standard though.

## work status

we do our best to release working code. we hacked this out pretty quickly so the code's quality is not all that right now.

status today is: *"it works, and the happy path is pretty solid. deviate from the path and there be dragons"*

## outline tasks

- [X] load local repository
- [X] enable open review of code
- [X] output in json
- [X] output in html
- [X] packaging so user can either install via `cargo install` or download the binary (macos apple silicon only)
- [X] (fine) tune the prompts for clarity and accuracy
- [X] more configuration and adjustment of prompts
- [X] enable google gemini review of code
- [ ] enable a private google gemini review of code using vertex ai (coming soon)
- [ ] github actions integration (coming soon)
- [X] enable private llm review of code (likely llama-based) run on a cloud service. (not fully tested, but wired in to use lm studio)
- [ ] better collation of static data from `git` and the abstract source tree (ast) to feed the generative ai
- [ ] proper documentation
- [ ] gitlab pipeline integration
- [ ] make adding other providers easy and robust - e.g, a anthropic claud review of code
- [ ] comparison of different llms review output on same code (this could be very cool!)

`>_ we are cosmonaut`

*copyright &#169; cosmonaut (new zealand) ltd, 2023*
