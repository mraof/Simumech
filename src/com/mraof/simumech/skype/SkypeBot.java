package com.mraof.simumech.skype;

import com.skype.Profile;
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
		(new Thread(new SkypeContactAdder(this))).start();
		
		try {
			Skype.getProfile().setStatus(Profile.Status.SKYPEME);
			Skype.addChatMessageListener(listener);
		} catch (SkypeException e) {e.printStackTrace();}
	}
	public void quit()
	{
		try {
			Skype.getProfile().setStatus(Profile.Status.INVISIBLE);
		} catch (SkypeException e) {e.printStackTrace();}
		running = false;
//		listener.messages.add
	}
}
