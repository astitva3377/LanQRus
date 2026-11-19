// TODO: Verify

const MAGIC: [u8; 2] = *b"LQ";
const VERSION: u8 = 1;

const APPROVE: u8 = 1;
const REJECT: u8 = 2;

pub(crate) const APPROVAL_MESSAGE: [u8; 4] =
	[MAGIC[0], MAGIC[1], VERSION, APPROVE];

pub(crate) const REJECTION_MESSAGE: [u8; 4] =
	[MAGIC[0], MAGIC[1], VERSION, REJECT];

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ControlMessage {
	APPROVE,
	REJECT,
}

pub(crate) fn decode_control_message(
	message: &[u8],
) -> Option<ControlMessage> {
	if message.len() != 4 {
		return None;
	}

	if message[0..2] != MAGIC {
		return None;
	}

	if message[2] != VERSION {
		return None;
	}

	match message[3] {
		APPROVE => Some(ControlMessage::APPROVE),
		REJECT => Some(ControlMessage::REJECT),
		_ => None,
	}
}
