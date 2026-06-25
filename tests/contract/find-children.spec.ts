/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract findChildren', () => {
	it('should find nothing given no properties', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);

		expect(container.findChildren(Contract.createMatcher({}))).to.deep.equal(
			[],
		);
	});

	it('should find a specific unique contract based on its type and name', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					name: 'Debian Wheezy',
					type: 'sw.os',
				}),
			),
		).to.deep.equal([contract1]);
	});

	it('should find a specific unique contract based on its type and slug', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract4 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'sw.os',
					slug: 'fedora',
				}),
			),
		).to.deep.equal([contract3]);
	});

	it('should find a specific unique contract based on another property', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract4 = new Contract(CONTRACTS['hw.device-type'].artik10.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					arch: 'armv7hf',
				}),
			),
		).to.deep.equal([contract4]);
	});

	it('should find multiple contracts based on a type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract4 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'sw.os',
				}),
			),
		).to.deep.equal([contract1, contract2, contract3]);
	});

	it('should find nothing based on a non-existent type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract4 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'non-existent-type',
				}),
			),
		).to.deep.equal([]);
	});

	it('should find nothing because of an invalid type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract4 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'non-existent-type',
					slug: 'debian',
				}),
			),
		).to.deep.equal([]);
	});

	it('should find a contract based on one of its aliases', () => {
		const contract1 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi 2',
			slug: 'raspberrypi2',
			aliases: ['rpi2', 'raspberry-pi2'],
		});

		const contract2 = new Contract({
			type: 'hw.device-type',
			name: 'Raspberry Pi',
			slug: 'raspberrypi',
			aliases: ['rpi', 'raspberry-pi'],
		});

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'rpi',
				}),
			),
		).to.deep.equal([contract2]);
	});

	it('should find a nested contract by its type and slug', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		contract1.addChild(contract3);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'sw.blob',
					slug: 'nodejs',
				}),
			),
		).to.deep.equal([contract3]);
	});

	it('should find a nested contract by its type and another property', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		contract1.addChild(contract3);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'sw.blob',
					version: '4.8.0',
				}),
			),
		).to.deep.equal([contract3]);
	});

	it('should fail to find a nested contract with an incorrect slug', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		contract1.addChild(contract3);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'sw.blob',
					slug: 'jtest',
				}),
			),
		).to.deep.equal([]);
	});

	it('should fail to find a nested contract with an incorrect type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		contract1.addChild(contract3);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'sw.os',
					slug: 'nodejs',
				}),
			),
		).to.deep.equal([]);
	});

	it('should be able to find a two level nested children using its type', () => {
		const contract1 = new Contract(CONTRACTS['hw.device-type'].artik10.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		contract2.addChild(contract3);
		contract1.addChild(contract2);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1]);

		expect(
			container.findChildren(
				Contract.createMatcher({
					type: 'sw.blob',
				}),
			),
		).to.deep.equal([contract3]);
	});
});
