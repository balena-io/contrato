info: {
	title:   "Contracts Specification"
	version: "1.0.0"
	license: {
		name: "Apache 2.0"
		url:  "https://www.apache.org/licenses/LICENSE-2.0.html"
	}
}

// Semantic Versioning compliant version (see https://semver.org)
//
// Example: `3.1.5`
#SemVer: string

// Semantic Version range
//
// See https://github.com/npm/node-semver#ranges
//
// Example: `>=1.0.0`
#SemVerRange: string

// A namespaced type string
#Type: =~"^[a-zA-Z][a-zA-Z0-9.-]*$"

// A slug string
#Slug: =~"^[a-zA-Z][a-zA-Z0-9-.]*$"

// Uniform resource location
#URL: "^(http|https|file)://.+$"

#Asset: {
	name?:         string
	url:           #URL
	checksum?:     string
	checksumType?: string

	// Validation: If checksum is present, checksumType must also be present
	// (this doesn't translate to openapi)
	if checksum != _|_ {
		checksumType: string
	}
}

// A matcher to a contract or a range of contracts
//
// Examples:
// Match all hw.device-type contracts with the given data
// ```yml
// type: hw.device-type
// data:
//     arch: armv7hf
//     hdmi: true
// ```
//
// Match all alpine versions bigger than 3.15
// ```yml
// type: sw.os
// slug: alpine
// version: >=3.15
// ```
#ContractMatcher: {
	#Contract
	version?: #SemVerRange
}

// A contract requirement
#ContractRequirement: #ContractMatcher | {or: [...#ContractRequirement]} | {not: [...#ContractRequirement]}

// The contract metadata specification
#ContractMetadata: {
	// A semver version of the entity definition as we have defined it in the contract. The version should be updated every time the contract information changes.
	//
	// If not provided, we assume the contract is version 1.0.0
	//
	// Example: `1.0.1`
	version?: #SemVer
	// A human readable name of the entity.
	//
	// Example: `Raspberry PI 3`
	name?: string
	// A human readable description of the entity
	//
	// Example: `Single-board device to enable your IoT projects`
	description?: string
	// Alternative, globally unique slugs
	//
	// Example: `[ 'rpi3', 'raspberry-pi3' ]`
	aliases?: [...#Slug]
	// A free-form object for contract specific information. Notice that contracts are not allowed to define any extra top-level properties, so any information specific to a type must live inside data
	data: {...}
	// The assets this contract requires.
	// There are two types of assets:
	// - Local (declared with a file path)
	// - Remote (declared with a URL)
	//
	// If the protocol prefix is not provided, `file://` is assumed. Slashes should be used as path separators (UNIX style).
	// The url data property is mandatory.
	// If name is not provided, the asset key can be used as a substitute.
	// The checksum property is optional, but if present, checksumType must exist.
	//
	// Example:
	// ```yml
	// assets:
	//   bin:
	//     name: qemu-arm-static
	//     url: file://./assets/qemu-arm-static
	//    checksum: 7bce65c956bbddbf83a8ce9121b505657e835df4a064823de51623858c25090f
	//     checksumType: sha256
	// ```
	assets: {
		[string]: #Asset
	}
	// Enables each contract to specify its requirements on the environment in order to be valid.
	// The requirements are specified as a contract reference or an operation (`or`,`not`) on requirements
	//
	// Example:
	// ```yml
	// type: sw.application
	// slug: balena-sound
	// requires:
	//   - or:
	//     - type: hw.connector
	//       slug: hdmiv1.5
	//     - type: hw.connector
	//       slug: usb3
	// ```
	requires?: [...#ContractRequirement]
	// Allows to specify what functionalities
	// or capabilities from the environment an entity defined by the contract provides.
	//
	// Differently from requirements, only a list of contract references is supported for now
	//
	// Example:
	// ```yml
	// type: sw.application
	// slug: balena-os-for-raspberrypi3
	// provides:
	//     - type: sw.os
	//       slug: balenaos
	// ```
	provides?: [...#ContractMatcher]
	// Allows to specify contract alternatives for different sets of requirements.
	//
	// It can be combined with templating to generate a large number of contracts
	// from a short specification
	// For an example, see: https://github.com/balena-io/contracts/blob/master/contracts/sw.stack/node/contract.json
	variants?: [...#ContractMetadata]
	// A contract can contain other contracts, which makes it a composite contract.
	// This is accomplished by adding other contracts inside the `children` property
	children?: [...#Contract]
}

// A contract is a specification for describing _things_. A thing can be pretty much anything,
// a software library, a feature, an API, etc. Relationships between things can be established
// via composition and referencing (`requires` and `provides`).
//
// Example:
// ```json
// {
//   "slug": "raspberrypi3",
//   "version": "1",
//   "type": "hw.device-type",
//   "aliases": [],
//   "name": "Raspberry Pi 3",
//   "assets": {
//     "logo": {
//       "url": "./raspberrypi3.svg",
//       "name": "logo"
//     }
//   },
//   "data": {
//     "arch": "armv7hf",
//     "hdmi": true,
//     "led": true,
//     "connectivity": {
//       "bluetooth": true,
//       "wifi": true
//     },
//     "storage": {
//       "internal": false
//     },
//     "media": {
//       "defaultBoot": "sdcard",
//       "altBoot": ["usb_mass_storage", "network"]
//     },
//     "is_private": false
//   }
// }
// ```
#Contract: {
	// The type of a contract, which mostly aims to determine the contents of the free-form data object.
	// Ideally types should be namespaced to avoid clashing of contract types.
	//
	// Example: `hw.device-type`
	type: #Type
	// Unique identifier for the contract. No two contracts of the same type should have the same slug.
	//
	// Example: `raspberrypi3`
	slug?: #Slug

	// The contract body
	#ContractMetadata
}

// A cardinality operator
//
// A cardinality is usually represented with a tuple that defines a range of
// integers. On top of that, the following syntax sugar is supported.
// Assuming `x` in an integer:
// - `x` -> `[ x, x ]`
// - `*` -> `[ 0, Infinity ]`
// - `?` -> `[ 0, 1 ]`
// - `1?` -> `[ 0, 1 ]`
// - `'x'` -> `[ x, x ]`
// - `x+` -> `[ x, Infinity ]`
// - `[ x, '*' ]` -> `[ x, Infinity ]`
#Cardinality: string

// A JSON schema filter
//
// Example
// ```json
// {
//   "type": "object",
//   "properties": {
//     "slug": {
//       "const": "armv7hf"
//     }
//   }
// }
// ```
#FilterSchema: {...}

// A set of cardinality operators for a blueprint
#BlueprintLayout: {
	[string]: #Cardinality | {cardinality: #Cardinality, filter: #FilterSchema}
}

// A blueprint is a partial contract that defines how to generate a certain combination of contracts
// from a universe. The result of "applying" a blueprint on a universe is a set of contexts.
#Blueprint: {
	type: "meta.blueprint"
	// The query for the blueprint using a set of cardinality operators
	//
	// Example
	// ```yml
	// selector:
	//   sw.os: '1'
	//   sw.blob: '*',
	//   arch.sw:
	//     cardinality: [0,1]
	//     filter:
	//       type: object
	//       properties:
	//         slug:
	//            const: armv7hf
	// ```
	layout: #BlueprintLayout
	// An object describing how the resulting contexts should look like. You may use properties such as type, slug, data, etc.
	// You may use blueprint results to construct certain properties by accessing children through the children property.
	skeleton?: {...}
}
