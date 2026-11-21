const MAGIC: [u8; 2] = *b"LQ";
const VERSION: u8 = 1;

const APPROVE: u8 = 1;
const REJECT: u8 = 2;
const REJECT_ACK: u8 = 3;
const APPROVE_ACK: u8 = 4;

pub(crate) const APPROVAL_MESSAGE: [u8; 4] =
	[MAGIC[0], MAGIC[1], VERSION, APPROVE];

pub(crate) const REJECTION_MESSAGE: [u8; 4] =
	[MAGIC[0], MAGIC[1], VERSION, REJECT];

pub(crate) const REJECTION_ACK_MESSAGE: [u8; 4] =
	[MAGIC[0], MAGIC[1], VERSION, REJECT_ACK];

pub(crate) const APPROVAL_ACK_MESSAGE: [u8; 4] =
	[MAGIC[0], MAGIC[1], VERSION, APPROVE_ACK];

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ControlMessage {
	Approve,
	Reject,
	RejectAck,
	ApproveAck,
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
		APPROVE => Some(ControlMessage::Approve),
		REJECT => Some(ControlMessage::Reject),
		REJECT_ACK => Some(ControlMessage::RejectAck),
		APPROVE_ACK => Some(ControlMessage::ApproveAck),
		_ => None,
	}
}
