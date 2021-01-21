# Better Ops

A badly written (for now) cli helper to manage multiple well-defined bash profiles, written in a single night self-hackathon.

## Usage

### Installation

```
cargo install betterops
```

## FAQs

### Could you use bash aliases to accomplish the same functionality?

Yup.

### Where are my profile configurations stored at?

Most likely `~/.betterops`.

### What should I do if I want to use sensitive environment variables for my profile

Preferably, you should only use `command` types for sensitive values, and either retrieve temporary credentials dynamically, or `cat` the values from a secret file.
