/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';
import * as _ from 'lodash';
import { hashObject } from '../../lib/hash';

describe('hashObject', () => {
	it('should return a string', () => {
		expect(
			hashObject({
				foo: 'bar',
			}),
		).to.be.a('string');
	});

	it('should not care about properties order', () => {
		const hash1 = hashObject({
			foo: 'bar',
			bar: 'baz',
		});

		const hash2 = hashObject({
			bar: 'baz',
			foo: 'bar',
		});

		expect(hash1).to.deep.equal(hash2);
	});

	it('should not rely on object references', () => {
		const object = {
			foo: 'bar',
		};

		const hash1 = hashObject(_.cloneDeep(object));
		const hash2 = hashObject(_.cloneDeep(object));
		const hash3 = hashObject(_.cloneDeep(object));

		expect(hash1).to.deep.equal(hash2);
		expect(hash2).to.deep.equal(hash3);
		expect(hash3).to.deep.equal(hash1);
	});

	it('should return different hashes for different objects', () => {
		const hash1 = hashObject({
			foo: 'bar',
		});

		const hash2 = hashObject({
			foo: 'baz',
		});

		const hash3 = hashObject({
			foo: 'qux',
		});

		expect(hash1).to.not.deep.equal(hash2);
		expect(hash2).to.not.deep.equal(hash3);
		expect(hash3).to.not.deep.equal(hash1);
	});
});
