/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract getChildrenByType', () => {
	it('should return all contracts of the type if the type exists', () => {
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
			container
				.getChildrenByType('sw.os')
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal(
			[contract1, contract2, contract3].map((c) => c.hash()).sort(),
		);
	});

	it('should always return the same results', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract4 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		const expectedHashes = [contract1, contract2, contract3]
			.map((c) => c.hash())
			.sort();

		expect(
			container
				.getChildrenByType('sw.os')
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal(expectedHashes);

		expect(
			container
				.getChildrenByType('sw.os')
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal(expectedHashes);

		expect(
			container
				.getChildrenByType('sw.os')
				.map((c) => c.hash())
				.sort(),
		).to.deep.equal(expectedHashes);
	});

	it('should return nothing if the type does not exist', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract4 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		expect(container.getChildrenByType('arch.sw')).to.deep.equal([]);
	});

	it('should not return the same contract multiple times if it contains aliases', () => {
		const contract1 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
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
			container.getChildrenByType('hw.device-type').map((c) => c.hash()),
		).to.deep.equal([contract2.hash()]);
	});
});
