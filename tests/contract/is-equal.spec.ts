/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';

describe('Contract isEqual', () => {
	it('should return true if the contracts are equal', () => {
		const contract1 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		const contract2 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		expect(Contract.isEqual(contract1, contract2)).to.be.true;
	});

	it('should return false if the contracts are different', () => {
		const contract1 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		const contract2 = new Contract({
			type: 'arch.sw',
			name: 'i386',
			slug: 'i386',
		});

		expect(Contract.isEqual(contract1, contract2)).to.be.false;
	});
});
