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

[![Documentation](https://github.com/balena-io/contrato/actions/workflows/docs.yml/badge.svg)](https://balena-io.github.io/contrato/modules/contrato.html)

## About contracts

### What is a contract?

Is a specification for describing _things_. A thing can be pretty much anything, a software library, a feature, an API, etc. Relationships between things can be established via composition and referencing (`requires` and `provides`). Through this library, contracts can be validated, composed and combined.

### Why build this?

balena.io is a complex product with a great number of inter-conecting components. Each of the components have their own requisites, capabilities, and incompatibilities. Contracts are an effort to formally document those interfaces, and a foundation on which we can build advanced tooling to ultimately automate the process of the team, increase productivity, and remove the human element from tasks that can be performed better by a machine.

The concept of contracts is generic enough that it can be applied to seemingly unrelated scenarios, from base images and OS images, to device types and backend components. Re-using the same contract "format" between them allows us to multiply the gains we get by developing complex contract-related programming modules.

### What can I do with contracts? Give me some examples

Describe a _thing_ via a contract

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

Validate requirements of a contract via [contrato](https://github.com/balena-io/contrato)

```ts
import { Contract } from 'contrato';

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

const curlContract = new Contract({
	type: 'sw.utility',
	slug: 'curl',
	version: '8.11.1',
	requires: [{ type: 'sw.library', slug: 'glibc', version: '>=2.17' }],
});

if (osContract.satisfiesChildContract(curlContract)) {
	console.log('cURL requirements are met and it can be installed!');
} else {
	// cannot install cURL, missing requirements: { type: 'sw.library', slug: 'glibc', version: '>=2.17' }
	console.log(
		'cannot install cURL, missing requirements: ',
		osContract.getNotSatisfiedChildRequirements(curlContract),
	);
}
```

Describe a universe of _things_

```ts
import { Contract, Universe } from 'contrato';

const universe = new Universe();
universe.addChildren([
	new Contract({ type: 'sw.os', slug: 'debian' }),
	new Contract({ type: 'sw.os', slug: 'fedora' }),
	new Contract({
		type: 'arch.sw',
		slug: 'armv7hf',
		requires: [{ type: 'hw.device-type', data: { arch: 'armv7hf' } }],
	}),
	new Contract({
		type: 'arch.sw',
		slug: 'amd64',
		requires: [{ type: 'hw.device-type', data: { arch: 'amd64' } }],
	}),
	new Contract({
		type: 'hw.device-type',
		slug: 'raspberrypi3',
		data: { arch: 'armv7hf' /* ... */ },
	}),
	new Contract({
		type: 'hw.device-type',
		slug: 'intel-nuc',
		data: { arch: 'amd64' /* ... */ },
	}),
]);
```

Generate combinations of _things_ with a Blueprint

```ts
import { Contract, Universe, Blueprint } from 'contrato';

const universe = new Universe();
universe.addChildren([
	/* ... */
]);

const blueprint = new Blueprint(
	{ 'hw.device-type': 1, 'arch.sw': 1, 'sw.os': 1 },
	{ type: 'meta.context' },
);

// Generate contexts with valid combinations of the given types
const contexts = blueprint.reproduce(universe);
```

Build templates using the metadata from a combination

````ts
import { Contract, Universe, Blueprint, buildTemplate } from 'contrato';

/* ... */

// Generate contexts with valid combinations of the given types
const contexts = blueprint.reproduce(universe);
const template = ```
Welcome to {{this.sw.os.slug}}OS for {{this.hw.device-type.slug}}!

This build supports the architecture {{this.arch.sw.slug}}
```;

for (const context of contexts) {
	// Welcome to OS fedoraOS for intel-nuc
	// ...
	console.log(buildTemplate(template, context));
}
````

### Additional information

See [contracts specification](balena-contracts.md) for additional documentation on the contract format.

## Tests

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
