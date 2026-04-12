import { expect } from '../chai';

import Contract from '../../lib/contract';

describe('Contract hash', () => {
	it('should produce a deterministic hash', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		const hash = contract.hash();
		expect(hash).to.be.a('string').and.have.lengthOf(64);
		expect(contract.hash()).to.equal(hash);
	});

	it('should produce different hashes for different contracts', () => {
		const contract1 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		const contract2 = new Contract({
			type: 'arch.sw',
			name: 'armel',
			slug: 'armel',
		});

		expect(contract1.hash()).to.not.equal(contract2.hash());
	});

	it('should produce the same hash regardless of field order', () => {
		const contract1 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			version: '1',
			data: { foo: 'bar' },
		});

		const contract2 = new Contract({
			data: { foo: 'bar' },
			version: '1',
			slug: 'armv7hf',
			name: 'armv7hf',
			type: 'arch.sw',
		});

		expect(contract1.hash()).to.equal(contract2.hash());
	});

	it('should return a stable hash across calls', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		const originalHash = contract.hash();
		expect(contract.hash()).to.equal(originalHash);
	});
});
