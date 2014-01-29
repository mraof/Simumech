package com.mraof.simumech.skype;

import com.mraof.simumech.IChat;
import com.skype.Profile;
import com.skype.Skype;
import com.skype.SkypeException;

public class SkypeBot implements IChat
{
	SkypeListener listener;
	public boolean running = false;
	Thread listeningThread;
	Thread contactThread;
	
	public SkypeBot() 
	{
		running = true;
		
		listener = new SkypeListener(this);
		listeningThread = new Thread(listener);
		listeningThread.start();
		contactThread = new Thread(new SkypeContactAdder(this));
		contactThread.start();
		
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
		listeningThread.interrupt();
		contactThread.interrupt();
		try {
			listeningThread.join();
			contactThread.join();
		} catch (InterruptedException e) {e.printStackTrace();}
//		listener.messages.add
	}
	@Override
	public void message(String message) 
	{
		
	}
	@Override
	public void command(String message) 
	{
		
	}
}
