/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import MatcherCache from '../../lib/matcher-cache';
import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract addChildren', () => {
	it('should add a set of one contract', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1]);

		expect(container.metadata.children).to.deep.equal({
			typeMatchers: {},
			searchCache: new MatcherCache(),
			types: new Set(['sw.os']),
			byType: {
				'sw.os': new Set([contract1.metadata.hash]),
			},
			byTypeSlug: {
				'sw.os': {
					debian: new Set([contract1.metadata.hash]),
				},
			},
			map: {
				[contract1.metadata.hash]: contract1,
			},
		});

		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: contract1.raw,
				},
			},
		});
	});

	it('should ignore duplicates from contract sets', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract1, contract1]);

		expect(container.metadata.children).to.deep.equal({
			typeMatchers: {},
			searchCache: new MatcherCache(),
			types: new Set(['sw.os']),
			byType: {
				'sw.os': new Set([contract1.metadata.hash]),
			},
			byTypeSlug: {
				'sw.os': {
					debian: new Set([contract1.metadata.hash]),
				},
			},
			map: {
				[contract1.metadata.hash]: contract1,
			},
		});

		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: contract1.raw,
				},
			},
		});
	});

	it('should add a set of multiple contracts to an empty universe', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		expect(container.metadata.children).to.deep.equal({
			typeMatchers: {},
			searchCache: new MatcherCache(),
			types: new Set(['sw.os']),
			byType: {
				'sw.os': new Set([contract1.metadata.hash, contract2.metadata.hash]),
			},
			byTypeSlug: {
				'sw.os': {
					debian: new Set([contract1.metadata.hash, contract2.metadata.hash]),
				},
			},
			map: {
				[contract1.metadata.hash]: contract1,
				[contract2.metadata.hash]: contract2,
			},
		});

		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: [contract1.raw, contract2.raw],
					},
				},
			},
		});
	});

	it('should return the instance', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		expect(container.addChildren([contract1, contract2])).to.deep.equal(
			container,
		);
	});

	it('should return the instance if no contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		expect(container.addChildren()).to.deep.equal(container);
	});

	it('should re-hash the universe', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const hash = container.metadata.hash;
		container.addChildren([contract1, contract2]);
		expect(container.metadata.hash).to.not.equal(hash);
	});

	it('should not re-hash the universe if the rehash option is false', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const hash = container.metadata.hash;
		container.addChildren([contract1, contract2], {
			rehash: false,
		});

		expect(container.metadata.hash).to.equal(hash);
	});

	it('should add a contract of a new slug to an existing type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3]);

		expect(container.metadata.children).to.deep.equal({
			typeMatchers: {},
			searchCache: new MatcherCache(),
			types: new Set(['sw.os']),
			byType: {
				'sw.os': new Set([
					contract1.metadata.hash,
					contract2.metadata.hash,
					contract3.metadata.hash,
				]),
			},
			byTypeSlug: {
				'sw.os': {
					debian: new Set([contract1.metadata.hash, contract2.metadata.hash]),
					fedora: new Set([contract3.metadata.hash]),
				},
			},
			map: {
				[contract1.metadata.hash]: contract1,
				[contract2.metadata.hash]: contract2,
				[contract3.metadata.hash]: contract3,
			},
		});

		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: [contract1.raw, contract2.raw],
						fedora: contract3.raw,
					},
				},
			},
		});
	});

	it('should add two contracts of a new slug to an existing type', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].fedora['24'].object);
		const contract4 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		expect(container.metadata.children).to.deep.equal({
			typeMatchers: {},
			searchCache: new MatcherCache(),
			types: new Set(['sw.os']),
			byType: {
				'sw.os': new Set([
					contract1.metadata.hash,
					contract2.metadata.hash,
					contract3.metadata.hash,
					contract4.metadata.hash,
				]),
			},
			byTypeSlug: {
				'sw.os': {
					debian: new Set([contract1.metadata.hash, contract2.metadata.hash]),
					fedora: new Set([contract3.metadata.hash, contract4.metadata.hash]),
				},
			},
			map: {
				[contract1.metadata.hash]: contract1,
				[contract2.metadata.hash]: contract2,
				[contract3.metadata.hash]: contract3,
				[contract4.metadata.hash]: contract4,
			},
		});

		expect(container.raw).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: [contract1.raw, contract2.raw],
						fedora: [contract3.raw, contract4.raw],
					},
				},
			},
		});
	});
});
