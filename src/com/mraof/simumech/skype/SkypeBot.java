package com.mraof.simumech.skype;

import com.skype.ChatMessage;
import com.skype.Skype;
import com.skype.SkypeException;

public class SkypeBot 
{
	SkypeListener listener;
	public boolean running = false;
	
	public SkypeBot() 
	{
		running = true;
		
		listener = new SkypeListener(this);
		(new Thread(listener)).start();
		
		try {
			Skype.addChatMessageListener(listener);
		} catch (SkypeException e) {e.printStackTrace();}
	}
	public void quit()
	{
		running = false;
//		listener.messages.add
	}
}
