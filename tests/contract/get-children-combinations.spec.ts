/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

const toHashes = (combos: Contract[][]) =>
	combos.map((combo) => combo.map((c) => c.hash()).sort());

const sortCombos = (combos: string[][]) =>
	combos
		.map((c) => [...c].sort())
		.sort((a, b) => a.join(',').localeCompare(b.join(',')));

describe('Contract getChilredCombinations', () => {
	it('should throw if the type is not valid', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(() => {
			container.getChildrenCombinations({
				type: 'foo',
				from: 2,
				to: 2,
			});
		}).throw(
			'Invalid cardinality: 2 to 2. The number of foo contracts in the universe is 0',
		);
	});

	it('should return combinations of cardinality 1 for one contract', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			wheezy,
		]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 1,
			to: 1,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([[wheezy.hash()]]),
		);
	});

	it('should return combinations of cardinality 1 for two contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const jessie = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			wheezy,
			jessie,
		]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 1,
			to: 1,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([[wheezy.hash()], [jessie.hash()]]),
		);
	});

	it('should return combinations of cardinality 1 for three contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const jessie = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const fedora25 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			wheezy,
			jessie,
			fedora25,
		]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 1,
			to: 1,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([[wheezy.hash()], [jessie.hash()], [fedora25.hash()]]),
		);
	});

	it('should return combinations of cardinality 2 for two contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const jessie = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			wheezy,
			jessie,
		]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 2,
			to: 2,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([[wheezy.hash(), jessie.hash()]]),
		);
	});

	it('should return combinations of cardinality 2 for three contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const jessie = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const fedora25 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			wheezy,
			jessie,
			fedora25,
		]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 2,
			to: 2,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([
				[wheezy.hash(), jessie.hash()],
				[wheezy.hash(), fedora25.hash()],
				[jessie.hash(), fedora25.hash()],
			]),
		);
	});

	it('should throw if "from" is greater than "to"', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			new Contract(CONTRACTS['sw.os'].debian.wheezy.object),
			new Contract(CONTRACTS['sw.os'].debian.jessie.object),
		]);

		expect(() => {
			container.getChildrenCombinations({
				type: 'sw.os',
				from: 2,
				to: 1,
			});
		}).throw(
			'Invalid cardinality: 2 to 1. The starting point is greater than the ending point',
		);
	});

	it('should generate combinations from 1 to 2 for one contract', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			wheezy,
		]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 1,
			to: 2,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([[wheezy.hash()]]),
		);
	});

	it('should return combinations from 1 to 2 for two contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const jessie = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			wheezy,
			jessie,
		]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 1,
			to: 2,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([
				[wheezy.hash()],
				[jessie.hash()],
				[wheezy.hash(), jessie.hash()],
			]),
		);
	});

	it('should return combinations from 1 to 3 for two contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const jessie = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		container.addChildren([
			new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object),
			wheezy,
			jessie,
		]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 1,
			to: 3,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([
				[wheezy.hash()],
				[jessie.hash()],
				[wheezy.hash(), jessie.hash()],
			]),
		);
	});

	it('should return combinations from 1 to 3 for three contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const fedora25 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const wheezy = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const jessie = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		container.addChildren([fedora25, wheezy, jessie]);

		const result = container.getChildrenCombinations({
			type: 'sw.os',
			from: 1,
			to: 3,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([
				[fedora25.hash()],
				[wheezy.hash()],
				[jessie.hash()],
				[fedora25.hash(), wheezy.hash()],
				[fedora25.hash(), jessie.hash()],
				[wheezy.hash(), jessie.hash()],
				[fedora25.hash(), wheezy.hash(), jessie.hash()],
			]),
		);
	});

	it('should not consider aliases as separate contracts', () => {
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

		const result = container.getChildrenCombinations({
			type: 'hw.device-type',
			from: 1,
			to: 1,
		});

		expect(sortCombos(toHashes(result))).to.deep.equal(
			sortCombos([[contract1.hash()], [contract2.hash()]]),
		);
	});
});
