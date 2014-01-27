package com.mraof.simumech.irc;

import com.mraof.simumech.IChatMessage;

public class IRCChatMessage implements IChatMessage 
{
	String message;
	String owner;
	String split;
	
	@Override
	public String getMessage() 
	{
		return message;
	}

	@Override
	public String getOwner() {
		return owner;
	}

}
