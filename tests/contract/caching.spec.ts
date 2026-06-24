/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import MatcherCache from '../../lib/matcher-cache';
import Contract from '../../lib/contract';
import CONTRACTS from '../contracts.json';

describe('Contract caching', () => {
	it('should have an empty cache by default', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);
		expect(container.$metadata.children.searchCache).to.deep.equal(
			new MatcherCache(),
		);
	});

	it('should create an entry after a successful search', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		const matcher = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		container.findChildren(matcher);

		const cache = new MatcherCache();
		cache.add(matcher, [contract1]);

		expect(container.$metadata.children.searchCache).to.deep.equal(cache);
	});

	it('should be able to store multiple entries', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'fedora',
		});

		container.findChildren(matcher1);
		container.findChildren(matcher2);

		const cache = new MatcherCache();
		cache.add(matcher1, [contract1]);
		cache.add(matcher2, [contract2]);

		expect(container.$metadata.children.searchCache).to.deep.equal(cache);
	});

	it('should still store an entry if there were no results', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		const matcher = Contract.createMatcher({
			type: 'sw.os',
			slug: 'alpine',
		});

		container.findChildren(matcher);

		const cache = new MatcherCache();
		cache.add(matcher, []);

		expect(container.$metadata.children.searchCache).to.deep.equal(cache);
	});

	it('should honor a matcher over and over again', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		const matcher = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		expect(container.findChildren(matcher)).to.deep.equal([contract1]);
		expect(container.findChildren(matcher)).to.deep.equal([contract1]);
		expect(container.findChildren(matcher)).to.deep.equal([contract1]);
		expect(container.findChildren(matcher)).to.deep.equal([contract1]);
	});

	it('should clear the cache for a certain type if a contract is added', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const contract4 = new Contract(CONTRACTS['hw.device-type'].artik10.object);
		const contract5 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2, contract3, contract4]);

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.blob',
			slug: 'nodejs',
		});

		container.findChildren(matcher1);
		container.findChildren(matcher2);
		container.addChild(contract5);

		const cache = new MatcherCache();
		cache.add(matcher2, [contract3]);

		expect(container.$metadata.children.searchCache).to.deep.equal(cache);
	});

	it('should clear the cache for a certain type if a contract is removed', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract3 = new Contract(CONTRACTS['sw.blob'].nodejs['4.8.0'].object);
		const contract4 = new Contract(CONTRACTS['hw.device-type'].artik10.object);
		const contract5 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([
			contract1,
			contract2,
			contract3,
			contract4,
			contract5,
		]);

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		const matcher2 = Contract.createMatcher({
			type: 'sw.blob',
			slug: 'nodejs',
		});

		container.findChildren(matcher1);
		container.findChildren(matcher2);
		container.removeChild(contract5);

		const cache = new MatcherCache();
		cache.add(matcher2, [contract3]);

		expect(container.$metadata.children.searchCache).to.deep.equal(cache);
	});

	it('should clear the cache for a certain type if the removed contract did not exist', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].fedora['25'].object);
		const contract3 = new Contract(CONTRACTS['sw.os'].debian.sid.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		container.addChildren([contract1, contract2]);

		const matcher1 = Contract.createMatcher({
			type: 'sw.os',
			slug: 'debian',
		});

		container.findChildren(matcher1);
		container.removeChild(contract3);

		const cache = new MatcherCache();
		cache.add(matcher1, [contract1]);

		expect(container.$metadata.children.searchCache).to.deep.equal(cache);
	});
});
