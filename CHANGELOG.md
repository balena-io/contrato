# Change Log

All notable changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/).

## v0.0.6 - 2018-08-17

## 0.12.2 - 2025-01-23

* Remove docs folder [Felipe Lalanne]

## 0.12.1 - 2025-01-22

* Update library documentation [Felipe Lalanne]

## 0.12.0 - 2025-01-14

* Improve promised-handlebars types [Felipe Lalanne]
* Stop exporting parseCardinality function [Felipe Lalanne]
* Remove Blueprint.sequence [Felipe Lalanne]
* Remove non-iterable version of Blueprint.reproduce [Felipe Lalanne]
* Update object-hash to v3 [Felipe Lalanne]
* Update semver dependency [Felipe Lalanne]
* Update js-combinatorics to v2 [Felipe Lalanne]
* Update balena-lint to v9 [Felipe Lalanne]
* Remove dependency on cuelang for type generation [Felipe Lalanne]

## 0.11.0 - 2025-01-10

* Use p-map rather than custom concurrentForEach [Felipe Lalanne]
* Add class to work with a Universe of contracts [Felipe Lalanne]

## 0.10.0 - 2024-12-20

* Remove skhema as dependency [Felipe Lalanne]

## 0.9.5 - 2024-12-18

* Switch to promised-handlebars to fix dependencies [Felipe Lalanne]
* Update Node support to v20 and above [Felipe Lalanne]
* Update typescript to v5 and fix build errors [Felipe Lalanne]
* Fix test command to search all spec files [Felipe Lalanne]

## 0.9.4 - 2024-04-30

* NPM: Use legacy peer dependencies [Christina Ying Wang]
* Update @types/node to 20 [Christina Ying Wang]
* Up/downgrade handlebars and handlebars-async-helpers [Christina Ying Wang]
* Update mocha to v10 [Christina Ying Wang]
* Update @balena/lint to v8 [Christina Ying Wang]
* Move @types to devDependencies [Christina Ying Wang]
* Bump NPM to ^10 [Christina Ying Wang]

## 0.9.3 - 2024-04-30

* Update Node.js to v20 [Self-hosted Renovate Bot]

## 0.9.2 - 2023-04-05

* Various optimizations [Pagan Gazzard]

## 0.9.1 - 2023-04-04

* Improve typing of `query` function [Pagan Gazzard]

## 0.9.0 - 2023-04-03

* Switch to an async `buildTemplate` to avoid blocking fs operations [Pagan Gazzard]

## 0.8.0 - 2023-03-31

* Remove non iterable cartesian product calculation [Felipe Lalanne]
* Add option to query to return results as iterable [Felipe Lalanne]
* Add option to return iterable to Blueprint.reproduce [Felipe Lalanne]
* Add iteration versions of flatten and filter [Felipe Lalanne]
* Use depth first search to calculate cartesian product [Felipe Lalanne]
* Update typescript and ES target to use latest API [Felipe Lalanne]

## 0.7.2 - 2023-03-30

* Remove console.log commited by mistake [Felipe Lalanne]

## 0.7.1 - 2023-03-30

* Fix linting [Felipe Lalanne]
* Allow additional properties for type generation [Felipe Lalanne]
* Remove unused memfs dependency [Felipe Lalanne]
* Do not use package-lock [Felipe Lalanne]
* Read partials from filesystem instead of memfs [Felipe Lalanne]

## 0.7.0 - 2023-03-29

* Use flowzone docs instead of custom action [Felipe Lalanne]
* Setup flowzone [Felipe Lalanne]

## 0.6.5 - 2021-05-08

* Removed unnecessary ci files [Micah Halter]
* Added more strict types to cue definitions [Micah Halter]

## 0.6.4 - 2021-05-06

* add circleCI tests [Micah Halter]

## 0.6.3 - 2021-05-06

* Updated to new product-os repo location [Micah Halter]

## 0.6.2 - 2021-05-05

* Add linting of the scripts folder [Micah Halter]

## 0.6.1 - 2021-05-05

* align repo with official Balena typescript skeleton [Micah Halter]
* finished migration to Typescript and added cue type generation [Micah Halter]
* patch: use ts-migrate to convert to TypeScript [Thomas Manning]
* patch: Update ava from 0.22.0 to 3.15.0 [Thomas Manning]

## 0.6.0 - 2020-09-18

* Add support for a "not" operator in "requires" [Juan Cruz Viotti]

## 0.5.0 - 2020-08-05

* Remove handlebars-helpers to shrink bundle size [Pagan Gazzard]

## 0.4.0 - 2020-08-04


<details>
<summary> Update skhema to 5.x [Pagan Gazzard] </summary>

> ### skhema-5.3.2 - 2020-08-04
> 
> * Switch to typed-error [Pagan Gazzard]
> 
> ### skhema-5.3.1 - 2020-08-04
> 
> * Add .versionbot/CHANGELOG.yml for nested changelogs [Pagan Gazzard]
> 
> ### skhema-5.3.0 - 2020-05-05
> 
> * filter: Throw a custom error if the schema is invalid [Juan Cruz Viotti]
> 
> ### skhema-5.2.9 - 2019-12-12
> 
> * Add test to show .filter() not working correctly [StefKors]
> * When combining with baseSchema merge enum with AND operator [StefKors]
> 
> ### skhema-5.2.8 - 2019-11-27
> 
> * Ensure values in "enum" are unique [Juan Cruz Viotti]
> 
> ### skhema-5.2.7 - 2019-11-27
> 
> * filter: Correctly handle "enum" inside "anyOf" [Juan Cruz Viotti]
> 
> ### skhema-5.2.6 - 2019-11-19
> 
> * merge: Be explicit about additionalProperties [Juan Cruz Viotti]
> 
> ### skhema-5.2.5 - 2019-05-09
> 
> * Add a resolver for the const keyword [Lucian]
> 
> ### skhema-5.2.4 - 2019-04-15
> 
> * Configure AJV instances with an LRU cache [Juan Cruz Viotti]
> 
> ### skhema-5.2.3 - 2019-04-15
> 
> * Set addUsedSchema to false in all AJV instances [Juan Cruz Viotti]
> 
> ### skhema-5.2.2 - 2019-03-20
> 
> * Fix bug in scoreMatch when handling arrays [Lucian]
> 
> ### skhema-5.2.1 - 2019-03-19
> 
> * Fix bad require name and .only in tests [Lucian]
> 
> ### skhema-5.2.10 - Invalid date
> 
> * .filter(): Only match if the base schema matches [Lucian Buzzo]
> 
> ### skhema-5.2.0 - 2019-03-19
> 
> * Add ability to provide custom resolvers to merge() [Lucian]
> 
> ### skhema-5.1.1 - 2019-02-08
> 
> * Split up and optimize lodash dependencies [Lucian]
> 
> ### skhema-5.1.0 - 2019-01-08
> 
> * feature: Implement method for restricting a schema by another schema [Lucian Buzzo]
> 
> ### skhema-5.0.0 - Invalid date
> 
> * Remove ability to add custom keywords or formats [Lucian]
> 
> ### skhema-4.0.4 - Invalid date
> 
> * Improve performance of clone operations [Lucian]
> 
> ### skhema-4.0.3 - 2018-12-10
> 
> * Don't bust AJV cache [Lucian]
> 
> ### skhema-4.0.2 - 2018-12-10
> 
> * Add benchmark tests [Giovanni Garufi]
> 
> ### skhema-4.0.1 - 2018-12-04
> 
> * Recurse through nested `anyOf` statements when filtering [Lucian]
> 
> ### skhema-4.0.0 - 2018-12-03
> 
> * Treat undefined additionalProperties as true instead of false [Lucian]
> 
> ### skhema-3.0.1 - Invalid date
> 
> * stryker: Increase test timeout [Juan Cruz Viotti]
> * test: Configure Stryker for mutative testing [Juan Cruz Viotti]
> 
> ### skhema-3.0.0 - 2018-11-29
> 
> * Define additionalProperty inheritance in anyOf [Giovanni Garufi]
> * Formalising filtering logic [Lucian]
> * Added failing test case with mutation [Lucian]
> 
> ### skhema-2.5.2 - 2018-11-07
> 
> * hotfix: Make sure things that should be filtered are filtered [Juan Cruz Viotti]
> 
> ### skhema-2.5.1 - 2018-11-06
> 
> * filter: Force additionalProperties: true on match schemas [Juan Cruz Viotti]
> 
> ### skhema-2.5.0 - 2018-10-16
> 
> * Validate against just the schema if `options.schemaOnly` is true [Lucian Buzzo]
> 
> ### skhema-2.4.1 - 2018-10-09
> 
> * merge: When merging an empty array, return a wildcard schema [Lucian Buzzo]
> 
> ### skhema-2.4.0 - 2018-10-09
> 
> * validate: Make object optional [Lucian Buzzo]
> 
</details>

# v0.3.1
## (2020-08-04)

* Add .versionbot/CHANGELOG.yml for nested changelogs [Pagan Gazzard]

# v0.3.0
## (2020-07-17)

* Add logical operator support in templates [Stevche Radevski]

## 0.2.1 - 2019-08-22

* Fix typings module name and optional params [Cameron Diver]

## 0.2.0 - 2019-08-22

* Add typescript types to project [Cameron Diver]

## 0.1.0 - 2019-08-19

* Add circleci configuration file [Cameron Diver]
* Only perform a version check if child version is valid [Cameron Diver]
* Feature: Add support for 'latest' in selectors [Andreas Fitzek]
* Feature: Add sequence generation for Blueprints [Andreas Fitzek]
* Feature: Add support for skhema filters in blueprint selectors [Andreas Fitzek]

## v0.0.7 - 2018-10-19

* Partials: Return correct path combinations given 3 versioned contracts [Juan Cruz Viotti]
* Test: Add missing test case for find partial [Trong Nghia Nguyen]

- Add aliases support

## v0.0.5 - 2018-04-18

- Add a collection of Handlebars templates to the partial system

## v0.0.4 - 2018-04-09

- Support generating combinations of more than 31 contracts

## v0.0.3 - 2018-03-07

### Changed

- Add versioned contract fallback paths when finding partials

## v0.0.2 - 2017-10-13

### Changed

- Fix package.json entry point
