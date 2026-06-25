/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';

import Contract from '../../../lib/contract';
import ObjectSet from '../../../lib/object-set';

const createContractObjectSet = (contracts: Contract[]) => {
	const set = new ObjectSet();

	for (const contract of contracts) {
		set.add(contract, {
			id: contract.hash(),
		});
	}

	return set;
};

describe('Contract requirements', () => {
	it('should create a simple contract with empty requirements', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [],
		});

		expect(contract.metadata.requirements).to.deep.equal({
			matchers: {},
			types: new Set(),
			compiled: new ObjectSet(),
		});

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [],
		});
	});

	it('should create a contract with a single top level requirement', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [
				{
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				},
			],
		});

		expect(contract.metadata.requirements).to.deep.equal({
			matchers: {
				'hw.device-type': createContractObjectSet([
					Contract.createMatcher({
						type: 'hw.device-type',
						slug: 'raspberry-pi',
					}),
				]),
			},
			types: new Set(['hw.device-type']),
			compiled: createContractObjectSet([
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				}),
			]),
		});

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [
				{
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				},
			],
		});
	});

	it('should ignore duplicate top level requirements matchers', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [
				{
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				},
				{
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				},
			],
		});

		expect(contract.metadata.requirements).to.deep.equal({
			matchers: {
				'hw.device-type': createContractObjectSet([
					Contract.createMatcher({
						type: 'hw.device-type',
						slug: 'raspberry-pi',
					}),
				]),
			},
			types: new Set(['hw.device-type']),
			compiled: createContractObjectSet([
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				}),
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				}),
			]),
		});

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [
				{
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				},
				{
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				},
			],
		});
	});

	it('should create a contract with two top level requirements', () => {
		const contract = new Contract({
			type: 'sw.os',
			name: 'Debian',
			slug: 'debian',
			requires: [
				{
					type: 'hw.device-type',
					slug: 'intel-nuc',
				},
				{
					type: 'arch.sw',
					slug: 'amd64',
				},
			],
		});

		expect(contract.metadata.requirements).to.deep.equal({
			matchers: {
				'hw.device-type': createContractObjectSet([
					Contract.createMatcher({
						type: 'hw.device-type',
						slug: 'intel-nuc',
					}),
				]),
				'arch.sw': createContractObjectSet([
					Contract.createMatcher({
						type: 'arch.sw',
						slug: 'amd64',
					}),
				]),
			},
			types: new Set(['hw.device-type', 'arch.sw']),
			compiled: createContractObjectSet([
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'intel-nuc',
				}),
				Contract.createMatcher({
					type: 'arch.sw',
					slug: 'amd64',
				}),
			]),
		});

		expect(contract.raw).to.deep.equal({
			type: 'sw.os',
			name: 'Debian',
			slug: 'debian',
			requires: [
				{
					type: 'hw.device-type',
					slug: 'intel-nuc',
				},
				{
					type: 'arch.sw',
					slug: 'amd64',
				},
			],
		});
	});

	it('should create a contract with a single or requirement', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [
				{
					or: [
						{
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						},
						{
							type: 'hw.device-type',
							slug: 'raspberry-pi2',
						},
					],
				},
			],
		});

		expect(contract.metadata.requirements).to.deep.equal({
			matchers: {
				'hw.device-type': createContractObjectSet([
					Contract.createMatcher({
						type: 'hw.device-type',
						slug: 'raspberry-pi',
					}),
					Contract.createMatcher({
						slug: 'raspberry-pi2',
						type: 'hw.device-type',
					}),
				]),
			},
			types: new Set(['hw.device-type']),
			compiled: createContractObjectSet([
				Contract.createMatcher(
					createContractObjectSet([
						Contract.createMatcher({
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						}),
						Contract.createMatcher({
							slug: 'raspberry-pi2',
							type: 'hw.device-type',
						}),
					]),
					{
						operation: 'or',
					},
				),
			]),
		});

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [
				{
					or: [
						{
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						},
						{
							type: 'hw.device-type',
							slug: 'raspberry-pi2',
						},
					],
				},
			],
		});
	});

	it('should ignore duplicate matchers from or requirements', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [
				{
					or: [
						{
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						},
						{
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						},
					],
				},
			],
		});

		expect(contract.metadata.requirements).to.deep.equal({
			matchers: {
				'hw.device-type': createContractObjectSet([
					Contract.createMatcher({
						type: 'hw.device-type',
						slug: 'raspberry-pi',
					}),
				]),
			},
			types: new Set(['hw.device-type']),
			compiled: createContractObjectSet([
				Contract.createMatcher(
					createContractObjectSet([
						Contract.createMatcher({
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						}),
						Contract.createMatcher({
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						}),
					]),
					{
						operation: 'or',
					},
				),
			]),
		});

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			requires: [
				{
					or: [
						{
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						},
						{
							type: 'hw.device-type',
							slug: 'raspberry-pi',
						},
					],
				},
			],
		});
	});
});
