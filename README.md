# Contrato

The official [contracts](#about-contracts) implementation

## Quickstart

```ts
import { Contract } from 'contrato';

const osContract = new Contract({
	type: 'sw.os',
	slug: 'balenaos',
	version: '6.1.2',
	children: [
		{ type: 'sw.service', slug: 'balena-engine', version: '20.10.43' },
		{ type: 'sw.service', slug: 'NetworkManager', version: '0.6.0' },
	],
	provides: [{ type: 'sw.feature', slug: 'secureboot' }],
});

const serviceContract = new Contract({
	type: 'sw.application',
	slug: 'myapp',
	requires: [
		{ type: 'sw.service', slug: 'balena-engine', version: '>20' },
		{ type: 'sw.feature', slug: 'secureboot' },
	],
});

if (osContract.satisfiesChildContract(serviceContract)) {
	console.log('myapp can be installed!');
}
```

[![API Documentation](https://github.com/balena-io/contrato/actions/workflows/docs.yml/badge.svg)](https://balena-io.github.io/contrato/modules/contrato.html)

## About contracts

Contracts provide a standardized mechanism to describing _things_. A thing generally refers to something versionable, e.g. a software library, a feature, an API, etc. Relationships between things can be established via composition and referencing (`requires` and `provides`). Through this library, contracts can be validated, composed and combined.

### Why build this?

balena.io is a complex product with a great number of inter-conecting components. Each of the components have their own requisites, capabilities, and incompatibilities. Contracts are an effort to formally document those interfaces, and a foundation on which we can build advanced tooling to ultimately automate the process of the team, increase productivity, and remove the human element from tasks that can be performed better by a machine.

The concept of contracts is generic enough that it can be applied to seemingly unrelated scenarios, from base images and OS images, to device types and backend components. Re-using the same contract syntax between them allows us to multiply the gains we get by developing complex contract-related programming modules.

### What can be done with contracts?

Describe a _thing_

```json
{
	"type": "sw.library",
	"slug": "glibc",
	"version": "2.40",
	"assets": {
		"license": {
			"name": "GNU Lesser General Public License",
			"url": "https://www.gnu.org/licenses/lgpl-3.0.html#license-text"
		}
	}
}
```

Describe a _thing_ that requires a _thing_

```json
{
	"type": "sw.utility",
	"slug": "curl",
	"version": "8.11.1",
	"requires": [{ "type": "sw.library", "slug": "glibc", "version": ">=2.17" }],
	"data": {
		"protocols": ["HTTP", "HTTPS", "FTP"]
	}
}
```

Describe a complex _thing_ via a composite contract

```json
{
	"type": "sw.os",
	"slug": "balenaos",
	"version": "4.1.5",
	"children": [
		{
			"type": "sw.library",
			"slug": "glibc",
			"version": "2.16",
			"assets": {
				"license": {
					"name": "GNU Lesser General Public License",
					"url": "https://www.gnu.org/licenses/lgpl-3.0.html#license-text"
				}
			}
		}
	]
}
```

Describe a set of things via [templating](#contract-templating)

```json
{
	"slug": "alpine",
	"type": "sw.os",
	"version": "1",
	"data": {
		"libc": "musl-libc",
		"latest": "3.20",
		"versionList": "`3.20 (latest)`, `3.19`"
	},
	"name": "Alpine Linux {{this.version}}",
	"requires": [{ "type": "sw.blob", "slug": "balena-idle" }],
	"variants": [
		{
			"requires": [
				{ "type": "sw.blob", "slug": "qemu" },
				{
					"or": [
						{ "type": "arch.sw", "slug": "armv7hf" },
						{ "type": "arch.sw", "slug": "rpi" },
						{ "type": "arch.sw", "slug": "aarch64" }
					]
				}
			],
			"variants": [{ "version": "3.19" }, { "version": "3.20" }]
		},
		{
			"requires": [
				{
					"or": [
						{ "type": "arch.sw", "slug": "i386" },
						{ "type": "arch.sw", "slug": "amd64" }
					]
				}
			],
			"variants": [{ "version": "3.19" }, { "version": "3.20" }]
		}
	]
}
```

### Additional information

See the [contracts specification](./balena-contracts.md) for additional documentation on the contract format.

## About contrato

Contrato is the Balena contracts implementation. It provides capabilities for searching, comparing, validating and cross referencing contracts, as well as generating combinations of the contracts from a given context (via [blueprints](#blueprints)) and using these combinations to build templates. Some additional description about these features is provided below.

### Contract validation

A contract is valid within a context if all requirements of the contract and its children are met in the given context. A requirement is met if there is a contract within the context (including children) that matches the requirement. For example

```ts
import { Contract } from 'contrato';
import assert from 'assert';

const osContract = new Contract({
	type: 'sw.os',
	slug: 'balenaos',
	version: '4.1.5',
	children: [
		{
			type: 'sw.library',
			slug: 'glibc',
			version: '2.16',
		},
	],
});

// This is true
assert(
	osContract.satisfiesChildContract(
		new Contract({
			type: 'sw.utility',
			slug: 'myapp',
			version: '8.11.1',
			requires: [{ type: 'sw.library', slug: 'glibc', version: '>=2.15' }],
		}),
	),
);

// This is false
assert(
	!osContract.satisfiesChildContract(
		new Contract({
			type: 'sw.utility',
			slug: 'myapp',
			version: '8.11.1',
			requires: [{ type: 'sw.library', slug: 'glibc', version: '<2' }],
		}),
	),
);
```

Contrato also allows to find unsatisfied requirements, e.g.

```ts
// Will print [{ type: 'sw.library', slug: 'glibc', version: '>=2.17' }]
console.log(
	osContract.getNotSatisfiedChildRequirements(
		new Contract({
			type: 'sw.utility',
			slug: 'curl',
			version: '8.11.1',
			requires: [{ type: 'sw.library', slug: 'glibc', version: '>=2.17' }],
		}),
	),
);
```

### Contract templating

String properties of contracts may reference other number or string properties declared on the same contract by using the `this` keyword along with handlebars notation.

For example

```json
{
	"slug": "mycontract",
	"version": "1.0.0",
	"name": "This is my contract",
	"aliases": ["my_contract"],
	"data": {
		"number": 5
	},
	"assets": {
		"file": "https://files.contracts.io/{{this.slug}}/{{this.componentVersion}}/{{this.data.number}}.data"
	}
}
```

A single template can be used to generate multiple contracts using the `variants` property on the Contract specification. A good example of this is in the [Alpine OS contract template](https://github.com/balena-io/contracts/blob/master/contracts/sw.os/alpine/contract.json) describes the combination of different architecture builds for a list of versions and can be compiled into a set of contracts. See also the [NodeJS contract](https://github.com/balena-io/contracts/blob/master/contracts/sw.stack/node/contract.json) for a more complex example.

In Contrato, the template can be compiled into concrete contracts using the [Contract.build](https://balena-io.github.io/contrato/classes/contrato.contract.html#build) function.

```ts
import { Contract } from 'contrato';

// Build the template into the resulting contracts
const contracts = Contract.build({ slug: 'mycontract' /* ... */ });
```

### Universes

A universe is a composite contract that conforms the collection of "things" being operated on. For instance, the set of contracts on [balena-io/contracts](https://github.com/balena-io/contracts) compose the universe of Balena's contracts containing the knowledge about device types, architectures, OS versions and software stacks available to Balena and its products.

Contrato provides the [Universe type](https://balena-io.github.io/contrato/classes/universe.html) for working with a universe of contracts.

```ts
import { Contract, Universe } from 'contrato';

// Load the universe of contracts from ./contracts
const universe = await Universe.fromFs('./contracts');

// Find all contracts for the Debian OS
const children = universe.findChildren(
	Contract.createMatcher({
		type: 'sw.os',
		slug: 'debian',
	}),
);
```

### Blueprints

A blueprint is a contract that defines how to generate a certain combination of contracts from a universe. The result of "applying" a blueprint on a universe is a set of contexts. A _context_ is a universe where all requirements are satisfied.

```ts
import { Universe, Blueprint } from 'contrato';

// Load the universe of contracts from balena-io/contracts
const universe = await Universe.fromFs('./contracts');

// Create a blueprint
const blueprint = new Blueprint(
	// specify a layout combining one instance of device-type, arch and os contracts
	{ 'hw.device-type': 1, 'arch.sw': 1, 'sw.os': 1 },
	// use this skeleton as the parent contract for each context
	{ type: 'meta.context' },
);

// Reproduce the blueprint over the universe
const contexts = blueprint.reproduce(universe);

for (const context of contexts) {
	// Do something with each context, e.g. build a template
}
```

### Templates

A contract (usually a context) can be combined with a template contract in order to produce a resulting text artifact based on template [Partials](#partials). Templates are rendered using the [handlebars templating language](https://handlebarsjs.com).

Example: generate OS install instructions from a context.

```
1. {{import partial="download" combination="sw.os"}}

2. {{import partial="download" combination="sw.image-writer"}}. {{import partial="flash" combination="sw.image-writer+hw.device-type"}}.

3. {{import partial="insert-install-media" combination="hw.device-type"}}. {{import partial="prepare-network" combination="hw.device-type"}}. {{import partial="boot-external" combination="hw.device-type"}}

{{#hw.device-type.storage.internal}}
4. The device should appear on you dashboard in a configuring state. {{import partial="description-of-internal-process" combination="sw.image-writer"}}. {{import partial="visual-appearance-when-off" combination="hw.device-type"}}

5. {{import partial="remove-install-media" combination="hw.device-type"}}. {{import partial="boot-internal" combination="hw.device-type"}}
{{/hw.device-type.storage.internal}}

6. Your device should appear here in the IDLE state in 30 seconds or so. Have fun!

**Troubleshooting:** If, upon boot, the device LED is blinking in groups of four, it is an indication that the device cannot connect to the internet. Please ensure the network adapter is functional.

{{#compare hw.device-type.media.installation "!==" "dfu"}}
**Pro tip:** You can repeat the initialisation steps for any amount of {{hw.device-type.name}}'s you have available, using the same {{hw.device-type.media.installation}}.
{{/compare}}
```

Contracts can be rendered with contrato using the [buildTemplate](https://balena-io.github.io/contrato/modules/contrato.html#buildtemplate) function

```ts
import { buildTemplate } from 'contrato';
import { promises as fs } from 'fs';

// Reproduce the blueprint over the universe
const contexts = blueprint.reproduce(universe);
const template = await fs.readFile('instructions.tpl', 'utf8');

for (const context of contexts) {
	console.log(await buildTemplate(template, context));
}
```

### Partials

Each contract or combination of contracts may have partials associated with it that may be assembled into text artifacts with the template system.

The convention is to have a directory of contracts with the following structure:

```
<type combinations>/<slug combinations>/contract.json
<type combinations>/<slug combinations>/partial1.tpl
<type combinations>/<slug combinations>/partial2.tpl
<type combinations>/<slug combinations>/partialN.tpl
```

The types and slugs fragments of the path might be one entity, or a set of entities separated by a + sign. For example:

```
sw.os+arch.sw/debian+amd64/installation.tpl
hw.device-type/ts4900/remove-install-media.tpl
```

The type combination section specifies the types of contracts that come into play for a particular partials subtree, separated by a `+` symbol. If the combination is `sw.os+arch.sw`, then it means that the subtree will take into account the combination of operating systems and architectures. Note that there can be combinations of a single type.

The slug combination section defines a subtree for a specific set of contracts that match the combination type. If the type combination is `sw.os+arch.sw`, a valid slug combination can be `debian+amd64`, which is the subtree that will be selected when matching the Debian GNU/Linux contract with the amd64 architecture contract.

Note that a slug combination may use `@` symbols to define subtrees for a specific version of one or more contracts in the combination. For example, `debian@wheezy+amd64` will be the subtree containing partials for the combination of Debian Wheezy and amd64.

You can also omit trailing portions of the slug combination to implement wildcards. If the type combination is `sw.os+arch.sw` and the slug combination is `debian`, it means that such subtree will apply to the combination of Debian GNU/Linux with any architecture.

The partial tree is then traversed from specific to general, until a match is found. This is the path that the contract system will follow when searching for the `download` template on the `sw.os+arch.sw` combination:

```
sw.os+arch.sw/<os>@<version>+<arch>@<version>/download.tpl
sw.os+arch.sw/<os>@<version>+<arch>/download.tpl
sw.os+arch.sw/<os>+<arch>@<version>/download.tpl
sw.os+arch.sw/<os>+<arch>/download.tpl
sw.os+arch.sw/<os>/download.tpl
```

Again, you can see some examples of partials at use on the [balena-io/contracts repository](https://github.com/balena-io/contracts/tree/master/contracts/sw.os%2Bhw.device-type/alpine%2Braspberrypi3).

### Limitations of contrato

Contrato is quite efficient at most tasks it performs, however most of the operations require that the validating context is stored in memory, which puts a limit to the size of the universe. Speed considerations become particularly evident when reproducing a blueprint over a large universe, where the more selectors, the larger the number of combinations and the processing time.

## Testing

Run the `test` npm script:

```sh
npm test
```

## Contribute

- Issue Tracker: [github.com/product-os/contrato/issues](https://github.com/balena-io/contrato/issues)
- Source Code: [github.com/product-os/contrato](https://github.com/balena-io/contrato)

Before submitting a PR, please make sure that you include tests, and that the
linter runs without any warning:

```sh
npm run lint
```

## Support

If you're having any problem, please [raise an
issue](https://github.com/balena-io/contrato/issues/new) on GitHub.

## License

The project is licensed under the Apache 2.0 license.
