import { expect } from '../chai';
import * as _ from 'lodash';

import Blueprint from '../../lib/blueprint';
import { parse } from '../../lib/cardinality';

describe('Blueprint constructor', () => {
	it('should parse a layout with one number selector', () => {
		const blueprint = new Blueprint({
			'hw.device-type': 1,
		});

		expect(blueprint.layout).to.deep.equal({
			types: new Set(['hw.device-type']),
			finite: {
				selectors: {
					'hw.device-type': [
						{
							cardinality: _.merge(parse([1, 1]), {
								type: 'hw.device-type',
							}),
							filter: undefined,
							version: undefined,
							type: 'hw.device-type',
						},
					],
				},
				types: new Set(['hw.device-type']),
			},
			infinite: {
				selectors: {},
				types: new Set(),
			},
		});
	});

	it('should parse a layout with one finite and one infinite selectors', () => {
		const blueprint = new Blueprint({
			'hw.device-type': 2,
			'arch.sw': '1+',
		});

		expect(blueprint.layout).to.deep.equal({
			types: new Set(['hw.device-type', 'arch.sw']),
			finite: {
				selectors: {
					'hw.device-type': [
						{
							cardinality: _.merge(parse([2, 2]), {
								type: 'hw.device-type',
							}),
							filter: undefined,
							version: undefined,
							type: 'hw.device-type',
						},
					],
				},
				types: new Set(['hw.device-type']),
			},
			infinite: {
				selectors: {
					'arch.sw': [
						{
							cardinality: _.merge(parse([1, Infinity]), {
								type: 'arch.sw',
							}),
							filter: undefined,
							version: undefined,
							type: 'arch.sw',
						},
					],
				},
				types: new Set(['arch.sw']),
			},
		});
	});

	it('should support object layout selectors', () => {
		const filterFunction = _.identity;

		const blueprint = new Blueprint({
			'hw.device-type': {
				cardinality: [2, 2],
			},
			'arch.sw': {
				cardinality: '1+',
				filter: filterFunction,
			},
		});

		expect(blueprint.layout).to.deep.equal({
			types: new Set(['hw.device-type', 'arch.sw']),
			finite: {
				selectors: {
					'hw.device-type': [
						{
							cardinality: _.merge(parse([2, 2]), {
								type: 'hw.device-type',
							}),
							filter: undefined,
							version: undefined,
							type: 'hw.device-type',
						},
					],
				},
				types: new Set(['hw.device-type']),
			},
			infinite: {
				selectors: {
					'arch.sw': [
						{
							cardinality: _.merge(parse([1, Infinity]), {
								type: 'arch.sw',
							}),
							filter: filterFunction,
							version: undefined,
							type: 'arch.sw',
						},
					],
				},
				types: new Set(['arch.sw']),
			},
		});
	});

	it('should allow passing a skeleton object', () => {
		const blueprint = new Blueprint(
			{
				'hw.device-type': 1,
			},
			{
				type: 'sw.os-image',
				name: 'Generic OS Image',
			},
		);

		expect(blueprint.skeleton).to.deep.equal({
			type: 'sw.os-image',
			name: 'Generic OS Image',
		});
	});
});
