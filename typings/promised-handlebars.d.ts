declare module 'promised-handlebars' {
	import type HandlebarsNS from 'handlebars';

	// Extend the Handlebars type to include an async `compile` method
	type Handlebars = typeof HandlebarsNS;

	// Convert the template delegate to async
	// Note: this was tested with handlebars 4.7.8
	type HandlebarsTemplateDelegate<T> = Handlebars.TemplateDelegate<T>;
	type HandlebarsTemplateDelegateArgs<T> = Parameters<
		HandlebarsTemplateDelegate<T>
	>;
	type HandlebarsTemplateDelegateReturn<T> = ReturnType<
		HandlebarsTemplateDelegate<T>
	>;
	type HandlebarsTemplateDelegateAsync<T> = (
		...args: HandlebarsTemplateDelegateArgs<T>
	) => Promise<HandlebarsTemplateDelegateReturn<T>>;

	// Create a new Handlebars interface with an updated compile function
	type CompileArgs = Parameters<Handlebars['compile']>;
	interface HandlebarsAsync extends Handlebars {
		compile: <T>(...args: CompileArgs) => HandlebarsTemplateDelegateAsync<T>;
	}

	// Define the function exported by the module
	const fn: (hb: Handlebars) => HandlebarsAsync;
	export = fn;
}
