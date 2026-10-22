# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Breaking changes

- **No longer generates `Debug`.** `#[derive(PhysAddr)]` now implements only
  the traits needed by `PhysAddr` and its users: `Clone`, `Copy`,
  `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Add` and `Sub`. Add `Debug` to the
  derive list explicitly if you need to format the address type:
  `#[derive(PhysAddr, Debug)]`.
