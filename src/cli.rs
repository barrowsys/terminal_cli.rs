use prelude::v1::*;
use property::*;
use terminal::*;
use autocomplete::*;
use cli_property::*;
use cli_command::*;

/// Helper for matching commands and properties against an input line.
pub struct CliExecutor<'a> {
	matcher: CliLineMatcher<'a>,
	terminal: &'a mut CharacterTerminalWriter
}

impl<'a> CliExecutor<'a> {
	pub fn new<T: CharacterTerminalWriter>(matcher: CliLineMatcher<'a>, terminal: &'a mut T) -> Self {
		CliExecutor {
			matcher: matcher,
			terminal: terminal
		}
	}

	/// Finish the execution of this line invocation.
	pub fn close(self) -> CliLineMatcher<'a> {
		self.matcher
	}

	/// Creates a new prefixed execution context, but only if the current line matches. Reduces the
	/// processing overhead for large tree command environments.
	pub fn with_prefix<'b, I: Into<Cow<'b, str>>>(&'b mut self, prefix: I) -> Option<PrefixedExecutor<'a, 'b>> {
		let prefix = prefix.into();
		if self.matcher.starts_with(&prefix) {
			let p = PrefixedExecutor {
				prefix: prefix,
				executor: self
			};

			return Some(p);
		} else {
			self.matcher.add_unmatched_prefix(&prefix);
		}
		
		None
	}

	/// Announces a command to be executed. Returns an execution context in case the command is invoked.
	pub fn run_command<'b>(&'b mut self, cmd: &str) -> Option<CommandContext<'b>> {

		if self.matcher.match_cmd_str(cmd, None) == LineMatcherProgress::MatchFound {
			let args = if let &LineBufferResult::Match { ref args, .. } = self.matcher.get_state() {
				Some(args.clone())
			} else {
				None
			};

			if let Some(args) = args {
				let ctx = CommandContext {
					args: args.into(),
					terminal: self.terminal,
					current_path: ""
				};
				
				return Some(ctx);
			}
		}

		None
	}
	
	/// Announces a property that can be manipulated. Returns an execution context in case the property
	/// is to be either retrieved or updated.
	pub fn run_property<'b, V, P, Id: Into<Cow<'b, str>>>(&'b mut self, property_id: Id, input_parser: P) -> Option<PropertyContext<'b, V>> where P: ValueInput<V>, V: Display {
		let property_id: Cow<str> = property_id.into();

		if self.matcher.match_cmd_str(&format!("{}/get", property_id), None) == LineMatcherProgress::MatchFound {
			let args = if let &LineBufferResult::Match { ref args, .. } = self.matcher.get_state() {
				args.clone()
			} else {
				"".into()
			};

			return Some(PropertyContext::Get(PropertyContextGet {
				common: PropertyContextCommon {
					args: args.into(),
					terminal: self.terminal,
					current_path: "",
					id: property_id,
					style: PropertyCommandStyle::DelimitedGetSet
				}
			}));
		}

		if self.matcher.match_cmd_str(&format!("{}/set", property_id), None) == LineMatcherProgress::MatchFound {
			let args = if let &LineBufferResult::Match { ref args, .. } = self.matcher.get_state() {
				args.trim()
			} else {
				"".into()
			};

			match input_parser.input(&args) {
				Ok(val) => {
					return Some(PropertyContext::Set(PropertyContextSet {
						common: PropertyContextCommon {
							args: args.into(),
							terminal: self.terminal,
							current_path: "",
							id: property_id,
							style: PropertyCommandStyle::DelimitedGetSet
						},
						value: val
					}));
				},
				Err(e) => {
					self.terminal.print_line(&format!("Couldn't parse the value: {}", e));
				}
			}
		}

		None
	}

	/// Get the associated terminal.
	pub fn get_terminal(&mut self) -> &mut CharacterTerminalWriter {
		self.terminal
	}
}

impl<'a> Deref for CliExecutor<'a> {
	type Target = &'a mut CharacterTerminalWriter;

    fn deref<'b>(&'b self) -> &'b &'a mut CharacterTerminalWriter {
        &self.terminal
    }
}

pub struct PrefixedExecutor<'a: 'p, 'p> {
	prefix: Cow<'p, str>,
	executor: &'p mut CliExecutor<'a>
}

impl<'a, 'p> PrefixedExecutor<'a, 'p> {
	fn add_prefix<'c>(&self, cmd: &'c str) -> String {
		format!("{}{}", self.prefix, cmd)
	}

	pub fn run_command<'b>(&'b mut self, cmd: &str) -> Option<CommandContext<'b>> {
		let cmd = self.add_prefix(cmd);

		self.executor.run_command(&cmd)
	}
	
	pub fn run_property<'b, V, P, Id: Into<Cow<'b, str>>>(&'b mut self, property_id: Id, input_parser: P) -> Option<PropertyContext<'b, V>> where P: ValueInput<V>, V: Display {
		let property_id: Cow<str> = property_id.into();
		let property_id = self.add_prefix(&property_id);

		self.executor.run_property(property_id, input_parser)
	}
}