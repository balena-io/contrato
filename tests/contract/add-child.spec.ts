/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import MatcherCache from '../../lib/matcher-cache';
import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract addChild', () => {
	it('should add a contract to a contract without children', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		container.addChild(contract1);

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

		// expect(container.raw, {
		//   type: 'foo',
		//   slug: 'bar',
		//   children: {
		//     sw: {
		//       os: contract1.raw
		//     }
		//   }
		// })
	});

	it('should add two contracts of different types', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.metadata.children).to.deep.equal({
			typeMatchers: {},
			searchCache: new MatcherCache(),
			types: new Set(['sw.os', 'sw.blob']),
			byType: {
				'sw.os': new Set([contract1.metadata.hash]),
				'sw.blob': new Set([contract2.metadata.hash]),
			},
			byTypeSlug: {
				'sw.os': {
					debian: new Set([contract1.metadata.hash]),
				},
				'sw.blob': {
					nodejs: new Set([contract2.metadata.hash]),
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
					os: contract1.raw,
					blob: contract2.raw,
				},
			},
		});
	});

	it('should not add a contract twice', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);

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

	it('should two contracts of same type but different slugs', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

		expect(container.metadata.children).to.deep.equal({
			typeMatchers: {},
			searchCache: new MatcherCache(),
			types: new Set(['sw.os']),
			byType: {
				'sw.os': new Set([contract1.metadata.hash, contract2.metadata.hash]),
			},
			byTypeSlug: {
				'sw.os': {
					debian: new Set([contract1.metadata.hash]),
					fedora: new Set([contract2.metadata.hash]),
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
						debian: contract1.raw,
						fedora: contract2.raw,
					},
				},
			},
		});
	});

	it('should add a new version of an existing contract', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);

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

	it('should add two new versions of an existing contract', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);
		const contract3 = new Contract(CONTRACTS['sw.os'].debian.sid.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChild(contract1);
		container.addChild(contract2);
		container.addChild(contract3);

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
					debian: new Set([
						contract1.metadata.hash,
						contract2.metadata.hash,
						contract3.metadata.hash,
					]),
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
						debian: [contract1.raw, contract2.raw, contract3.raw],
					},
				},
			},
		});
	});

	it('should return the instance', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		expect(container.addChild(contract1)).to.deep.equal(container);
	});

	it('should re-hash the parent contract', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const hash = container.hash();
		container.addChild(contract1);
		expect(container.hash()).to.not.equal(hash);
	});
});
