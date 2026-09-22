/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';

describe('Contract getChildrenCrossReferencedContracts', () => {
	it('should compute the intersection of one type and two contracts with or operators', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract({
				type: 'arch.sw',
				slug: 'armel',
				name: 'armel',
			}),
			new Contract({
				type: 'arch.sw',
				slug: 'armv7hf',
				name: 'armv7hf',
			}),
			new Contract({
				type: 'arch.sw',
				slug: 'i386',
				name: 'i386',
			}),
			new Contract({
				type: 'sw.os',
				name: 'Debian Wheezy',
				slug: 'debian',
				version: 'wheezy',
				requires: [
					{
						or: [
							{
								type: 'arch.sw',
								slug: 'armel',
							},
							{
								type: 'arch.sw',
								slug: 'armv7hf',
							},
						],
					},
				],
			}),
			new Contract({
				type: 'sw.stack',
				name: 'Node.js {{version}}',
				slug: 'nodejs',
				version: '4.8.0',
				requires: [
					{
						or: [
							{
								type: 'arch.sw',
								slug: 'armv7hf',
							},
							{
								type: 'arch.sw',
								slug: 'i386',
							},
						],
					},
				],
			}),
		]);

		const contracts = contract.getChildrenCrossReferencedContracts({
			types: new Set(['arch.sw']),
			from: contract,
		});

		expect(contracts).to.equalByHash([
			new Contract({
				type: 'arch.sw',
				slug: 'armv7hf',
				name: 'armv7hf',
			}),
		]);
	});

	it('should return nothing if the from contract does not contain the referenced contracts', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract({
				type: 'sw.os',
				name: 'Debian Wheezy',
				slug: 'debian',
				version: 'wheezy',
				requires: [
					{
						or: [
							{
								type: 'arch.sw',
								slug: 'armel',
							},
							{
								type: 'arch.sw',
								slug: 'armv7hf',
							},
						],
					},
				],
			}),
			new Contract({
				type: 'sw.stack',
				name: 'Node.js {{version}}',
				slug: 'nodejs',
				version: '4.8.0',
				requires: [
					{
						or: [
							{
								type: 'arch.sw',
								slug: 'armv7hf',
							},
							{
								type: 'arch.sw',
								slug: 'i386',
							},
						],
					},
				],
			}),
		]);

		const references = contract.getChildrenCrossReferencedContracts({
			types: new Set(['arch.sw']),
			from: contract,
		});

		expect(references).to.equalByHash([]);
	});

	it('should compute the intersection of more than one type, from more than two contracts', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract({
				type: 'arch.sw',
				slug: 'armel',
				name: 'armel',
			}),
			new Contract({
				type: 'arch.sw',
				slug: 'armv7hf',
				name: 'armv7hf',
			}),
			new Contract({
				type: 'arch.sw',
				slug: 'i386',
				name: 'i386',
			}),
			new Contract({
				slug: 'artik10',
				type: 'hw.device-type',
				name: 'Samsung Artik 10',
			}),
			new Contract({
				slug: 'raspberry-pi',
				type: 'hw.device-type',
				name: 'Raspberry Pi (1, Zero, Zero W)',
			}),
			new Contract({
				slug: 'raspberrypi3',
				type: 'hw.device-type',
				name: 'Raspberry Pi 3',
			}),
			new Contract({
				type: 'sw.os',
				name: 'Debian Wheezy',
				slug: 'debian',
				version: 'wheezy',
				requires: [
					{
						or: [
							{
								type: 'arch.sw',
								slug: 'armel',
							},
							{
								type: 'arch.sw',
								slug: 'armv7hf',
							},
						],
					},
					{
						type: 'hw.device-type',
						slug: 'raspberry-pi',
					},
				],
			}),
			new Contract({
				type: 'sw.stack',
				name: 'Node.js {{version}}',
				slug: 'nodejs',
				version: '4.8.0',
				requires: [
					{
						or: [
							{
								type: 'arch.sw',
								slug: 'armv7hf',
							},
							{
								type: 'arch.sw',
								slug: 'i386',
							},
						],
					},
					{
						or: [
							{
								type: 'hw.device-type',
								slug: 'raspberry-pi',
							},
							{
								type: 'hw.device-type',
								slug: 'artik10',
							},
							{
								type: 'hw.device-type',
								slug: 'raspberrypi3',
							},
						],
					},
				],
			}),
			new Contract({
				type: 'sw.variant',
				name: 'Slim',
				slug: 'slim',
				requires: [
					{
						type: 'arch.sw',
						slug: 'armv7hf',
					},
					{
						or: [
							{
								type: 'hw.device-type',
								slug: 'raspberry-pi',
							},
							{
								type: 'hw.device-type',
								slug: 'artik10',
							},
						],
					},
				],
			}),
		]);

		const references = contract.getChildrenCrossReferencedContracts({
			types: new Set(['arch.sw', 'hw.device-type']),
			from: contract,
		});

		expect(references).to.equalByHash([
			new Contract({
				type: 'arch.sw',
				slug: 'armv7hf',
				name: 'armv7hf',
			}),
			new Contract({
				slug: 'raspberry-pi',
				type: 'hw.device-type',
				name: 'Raspberry Pi (1, Zero, Zero W)',
			}),
		]);
	});

	it('should return nothing if there is no intersection', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract({
				type: 'arch.sw',
				slug: 'armel',
				name: 'armel',
			}),
			new Contract({
				type: 'arch.sw',
				slug: 'armv7hf',
				name: 'armv7hf',
			}),
			new Contract({
				type: 'arch.sw',
				slug: 'i386',
				name: 'i386',
			}),
			new Contract({
				type: 'sw.os',
				name: 'Debian Wheezy',
				slug: 'debian',
				version: 'wheezy',
				requires: [
					{
						or: [
							{
								type: 'arch.sw',
								slug: 'armel',
							},
							{
								type: 'arch.sw',
								slug: 'armv7hf',
							},
						],
					},
				],
			}),
			new Contract({
				type: 'sw.stack',
				name: 'Node.js {{version}}',
				slug: 'nodejs',
				version: '4.8.0',
				requires: [
					{
						type: 'arch.sw',
						slug: 'i386',
					},
				],
			}),
		]);

		const references = contract.getChildrenCrossReferencedContracts({
			types: new Set(['arch.sw']),
			from: contract,
		});

		expect(references).to.equalByHash([]);
	});

	it('should not discard contracts of a type not defined in another contracts', () => {
		const contract = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		contract.addChildren([
			new Contract({
				type: 'arch.sw',
				slug: 'armv7hf',
				name: 'armv7hf',
			}),
			new Contract({
				slug: 'tini',
				type: 'sw.blob',
				name: 'TINI',
				version: '0.14.0',
			}),
			new Contract({
				type: 'sw.os',
				name: 'Debian Wheezy',
				slug: 'debian',
				version: 'wheezy',
				requires: [
					{
						type: 'sw.blob',
						slug: 'tini',
					},
					{
						type: 'arch.sw',
						slug: 'armv7hf',
					},
				],
			}),
			new Contract({
				type: 'sw.stack',
				name: 'Node.js {{version}}',
				slug: 'nodejs',
				version: '4.8.0',
				requires: [
					{
						type: 'arch.sw',
						slug: 'armv7hf',
					},
				],
			}),
		]);

		const references = contract.getChildrenCrossReferencedContracts({
			types: new Set(['arch.sw', 'sw.blob']),
			from: contract,
		});

		expect(references).to.equalByHash([
			new Contract({
				type: 'arch.sw',
				slug: 'armv7hf',
				name: 'armv7hf',
			}),
			new Contract({
				slug: 'tini',
				type: 'sw.blob',
				name: 'TINI',
				version: '0.14.0',
			}),
		]);
	});
});
