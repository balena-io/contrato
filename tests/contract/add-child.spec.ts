/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

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

		expect(container.getChildren()).to.have.lengthOf(1);
		expect(container.raw()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: contract1.raw(),
				},
			},
		});
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

		expect(container.getChildren()).to.have.deep.members([
			contract1,
			contract2,
		]);
		expect(container.getChildrenTypes()).to.deep.equal(
			new Set(['sw.os', 'sw.blob']),
		);
		expect(container.getChildrenByType('sw.os')).to.deep.equal([contract1]);
		expect(container.getChildrenByType('sw.blob')).to.deep.equal([contract2]);

		expect(container.raw()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: contract1.raw(),
					blob: contract2.raw(),
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

		expect(container.getChildren()).to.deep.equal([contract1]);
		expect(container.getChildrenTypes()).to.deep.equal(new Set(['sw.os']));
		expect(container.getChildrenByType('sw.os')).to.deep.equal([contract1]);

		expect(container.raw()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: contract1.raw(),
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

		expect(container.getChildren()).to.have.deep.members([
			contract1,
			contract2,
		]);
		expect(container.getChildrenTypes()).to.deep.equal(new Set(['sw.os']));
		expect(container.getChildrenByType('sw.os')).to.have.deep.members([
			contract1,
			contract2,
		]);

		expect(container.raw()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: contract1.raw(),
						fedora: contract2.raw(),
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

		expect(container.getChildren()).to.have.deep.members([
			contract1,
			contract2,
		]);
		expect(container.getChildrenTypes()).to.deep.equal(new Set(['sw.os']));
		expect(container.getChildrenByType('sw.os')).to.have.deep.members([
			contract1,
			contract2,
		]);

		expect(container.raw()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: [contract1.raw(), contract2.raw()],
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

		expect(container.getChildren()).to.have.deep.members([
			contract1,
			contract2,
			contract3,
		]);
		expect(container.getChildrenTypes()).to.deep.equal(new Set(['sw.os']));
		expect(container.getChildrenByType('sw.os')).to.have.deep.members([
			contract1,
			contract2,
			contract3,
		]);

		expect(container.raw()).to.deep.equal({
			type: 'foo',
			slug: 'bar',
			children: {
				sw: {
					os: {
						debian: [contract1.raw(), contract2.raw(), contract3.raw()],
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

	it('should reject a child whose type extends an existing child type', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		container.addChild(contract1);
		const before = container.raw();
		const hash = container.hash();

		expect(() =>
			container.addChild(new Contract({ type: 'sw.os.kernel', slug: 'linux' })),
		).to.throw("'sw.os' is a prefix of 'sw.os.kernel'");

		expect(container.raw()).to.deep.equal(before);
		expect(container.hash()).to.equal(hash, 'a rejected child must not rehash');
	});

	it('should reject a child whose type is a prefix of an existing child type', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		container.addChild(contract1);

		expect(() =>
			container.addChild(new Contract({ type: 'sw', slug: 'os' })),
		).to.throw("'sw' is a prefix of 'sw.os'");

		expect(container.getChildren()).to.deep.equal([contract1]);
	});
});
