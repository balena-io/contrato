import { expect } from '../chai';

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

		expect(container.getChildren()).to.have.lengthOf(1);
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

		expect(container.getChildren()).to.have.lengthOf(1);
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

		expect(container.getChildren()).to.have.lengthOf(2);
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

		expect(container.addChildren([contract1, contract2])).to.equal(container);
	});

	it('should return the instance if no contracts', () => {
		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		expect(container.addChildren()).to.equal(container);
	});

	it('should change the hash of the universe', () => {
		const contract1 = new Contract(CONTRACTS['sw.os'].debian.wheezy.object);
		const contract2 = new Contract(CONTRACTS['sw.os'].debian.jessie.object);

		const container = new Contract({
			type: 'foo',
			slug: 'bar',
		});

		const hash = container.hash();
		container.addChildren([contract1, contract2]);
		expect(container.hash()).to.not.equal(hash);
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

		expect(container.getChildren()).to.have.lengthOf(3);
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

		expect(container.getChildren()).to.have.lengthOf(4);
		const json = container.raw;
		expect(json.children.sw.os.debian).to.have.lengthOf(2);
		expect(json.children.sw.os.fedora).to.have.lengthOf(2);
	});
});
